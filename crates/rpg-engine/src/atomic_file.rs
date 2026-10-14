//! Durable publication of a fully written and re-verified file.
//!
//! The save store, the options store, and the Python importer's slot backup all publish the same
//! way: write a uniquely named temporary file beside the destination with exclusive creation, sync
//! it, read it back for the caller's own verification, publish it in one directory operation, and
//! sync the directory. Each caller keeps its own validation, naming, and policy rules; this module
//! owns only the filesystem sequence and reports which side of publication a failure fell on.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

/// How the verified temporary file takes the destination's name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublishPolicy {
    /// Atomically replace any existing destination.
    Replace,
    /// Create the destination only if no directory entry, including a dangling symlink, holds
    /// its name. Linux links the verified inode atomically or refuses an occupied name; there is
    /// no fallback to replacement on filesystems that reject hard links.
    CreateNew,
}

/// One step of the sequence, in order. Steps before [`Self::Publish`] never touch the destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublishStep {
    CreateTemporary,
    WriteTemporary,
    SyncTemporary,
    ReadBackTemporary,
    Publish,
    /// [`PublishPolicy::CreateNew`] only: the temporary name is unlinked after the link succeeds.
    RemovePublishedTemporary,
    SyncDirectory,
}

/// What went wrong at one step.
#[derive(Debug)]
pub(crate) enum PublishFailure<E> {
    Io {
        step: PublishStep,
        error: io::Error,
    },
    /// [`PublishPolicy::CreateNew`] found the destination name occupied.
    DestinationExists,
    /// The caller's verification rejected the re-read bytes, or its hook stopped the sequence.
    Caller(E),
}

/// A failed publication, classified by what the destination holds afterwards.
#[derive(Debug)]
pub(crate) enum PublishError<E> {
    /// The destination is exactly as it was before the call.
    NotPublished(PublishFailure<E>),
    /// The destination already holds the verified bytes, but a later step failed, so the
    /// publication may not survive a crash or power loss.
    NotDurable(PublishFailure<E>),
}

impl<E> PublishError<E> {
    pub(crate) fn failure(self) -> PublishFailure<E> {
        match self {
            Self::NotPublished(failure) | Self::NotDurable(failure) => failure,
        }
    }
}

/// Where a file is published and under what policy.
pub(crate) struct Publication<'a> {
    /// The destination's directory; it must already exist.
    pub(crate) directory: &'a Path,
    pub(crate) destination: &'a Path,
    /// Distinguishes this destination's temporary files: `.{stem}.{pid}.{n}.tmp`.
    pub(crate) temporary_stem: &'a str,
    pub(crate) policy: PublishPolicy,
}

impl Publication<'_> {
    /// Writes `bytes`, verifies the re-read copy, and publishes it.
    ///
    /// `verify` receives the bytes read back from the synced temporary file. `before` runs ahead
    /// of the write, sync, publish, and directory-sync steps with the temporary and destination
    /// paths; production callers use it only for test fault injection and race simulation.
    pub(crate) fn publish<E>(
        &self,
        bytes: &[u8],
        verify: impl FnOnce(&[u8]) -> Result<(), E>,
        mut before: impl FnMut(PublishStep, &Path, &Path) -> Result<(), E>,
    ) -> Result<(), PublishError<E>> {
        use PublishError::{NotDurable, NotPublished};
        let io = |step| move |error| PublishFailure::Io { step, error };
        let temporary = self.directory.join(format!(
            ".{}.{}.{}.tmp",
            self.temporary_stem,
            std::process::id(),
            NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut cleanup = TemporaryFile::new(temporary.clone());
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(io(PublishStep::CreateTemporary))
            .map_err(NotPublished)?;
        let mut step =
            |step| before(step, &temporary, self.destination).map_err(PublishFailure::Caller);
        step(PublishStep::WriteTemporary).map_err(NotPublished)?;
        file.write_all(bytes)
            .map_err(io(PublishStep::WriteTemporary))
            .map_err(NotPublished)?;
        step(PublishStep::SyncTemporary).map_err(NotPublished)?;
        file.sync_all()
            .map_err(io(PublishStep::SyncTemporary))
            .map_err(NotPublished)?;
        drop(file);
        let mut verified = Vec::new();
        File::open(&temporary)
            .and_then(|mut file| file.read_to_end(&mut verified))
            .map_err(io(PublishStep::ReadBackTemporary))
            .map_err(NotPublished)?;
        verify(&verified)
            .map_err(PublishFailure::Caller)
            .map_err(NotPublished)?;
        step(PublishStep::Publish).map_err(NotPublished)?;
        match self.policy {
            PublishPolicy::Replace => {
                fs::rename(&temporary, self.destination)
                    .map_err(io(PublishStep::Publish))
                    .map_err(NotPublished)?;
            }
            PublishPolicy::CreateNew => {
                fs::hard_link(&temporary, self.destination)
                    .map_err(|error| {
                        if error.kind() == io::ErrorKind::AlreadyExists {
                            PublishFailure::DestinationExists
                        } else {
                            PublishFailure::Io {
                                step: PublishStep::Publish,
                                error,
                            }
                        }
                    })
                    .map_err(NotPublished)?;
                fs::remove_file(&temporary)
                    .map_err(io(PublishStep::RemovePublishedTemporary))
                    .map_err(NotDurable)?;
            }
        }
        cleanup.disarm();
        step(PublishStep::SyncDirectory).map_err(NotDurable)?;
        sync_directory(self.directory)
            .map_err(io(PublishStep::SyncDirectory))
            .map_err(NotDurable)
    }
}

/// Makes directory-entry changes (a create, link, rename, or unlink) durable.
pub(crate) fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory).and_then(|directory| directory.sync_all())
}

/// Removes a file it was armed for when dropped; disarm once the file has been handed off.
pub(crate) struct TemporaryFile {
    path: PathBuf,
    armed: bool,
}

impl TemporaryFile {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(PathBuf);

    impl Directory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "rpg-s1-atomic-file-{label}-{}-{}",
                std::process::id(),
                NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn entries(&self) -> Vec<String> {
            let mut names = fs::read_dir(&self.0)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect::<Vec<_>>();
            names.sort();
            names
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn publication<'a>(
        directory: &'a Directory,
        destination: &'a Path,
        policy: PublishPolicy,
    ) -> Publication<'a> {
        Publication {
            directory: &directory.0,
            destination,
            temporary_stem: "target",
            policy,
        }
    }

    fn accept(_: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }

    fn no_hook(_: PublishStep, _: &Path, _: &Path) -> Result<(), &'static str> {
        Ok(())
    }

    #[test]
    fn both_policies_publish_the_verified_bytes_and_leave_no_temporary_file() {
        for policy in [PublishPolicy::Replace, PublishPolicy::CreateNew] {
            let directory = Directory::new("publish");
            let destination = directory.0.join("target.yaml");
            let mut seen = None;
            publication(&directory, &destination, policy)
                .publish(
                    b"verified",
                    |bytes| {
                        seen = Some(bytes.to_vec());
                        Ok::<(), &str>(())
                    },
                    no_hook,
                )
                .unwrap();
            assert_eq!(seen.as_deref(), Some(b"verified".as_slice()));
            assert_eq!(fs::read(&destination).unwrap(), b"verified");
            assert_eq!(directory.entries(), ["target.yaml"]);
        }
    }

    #[test]
    fn replace_overwrites_and_create_new_refuses_an_occupied_name() {
        let directory = Directory::new("policy");
        let destination = directory.0.join("target.yaml");
        fs::write(&destination, b"incumbent").unwrap();
        let refused = publication(&directory, &destination, PublishPolicy::CreateNew)
            .publish(b"new", accept, no_hook)
            .unwrap_err();
        assert!(matches!(
            refused,
            PublishError::NotPublished(PublishFailure::DestinationExists)
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"incumbent");
        publication(&directory, &destination, PublishPolicy::Replace)
            .publish(b"new", accept, no_hook)
            .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert_eq!(directory.entries(), ["target.yaml"]);
    }

    #[test]
    fn a_rejected_read_back_or_stopped_step_publishes_nothing_and_cleans_up() {
        let directory = Directory::new("rejected");
        let destination = directory.0.join("target.yaml");
        let rejected = publication(&directory, &destination, PublishPolicy::Replace)
            .publish(b"bytes", |_| Err("rejected"), no_hook)
            .unwrap_err();
        assert!(matches!(
            rejected,
            PublishError::NotPublished(PublishFailure::Caller("rejected"))
        ));
        for stop in [
            PublishStep::WriteTemporary,
            PublishStep::SyncTemporary,
            PublishStep::Publish,
        ] {
            let stopped = publication(&directory, &destination, PublishPolicy::CreateNew)
                .publish(b"bytes", accept, |step, _, _| {
                    if step == stop { Err("stopped") } else { Ok(()) }
                })
                .unwrap_err();
            assert!(matches!(
                stopped,
                PublishError::NotPublished(PublishFailure::Caller("stopped"))
            ));
        }
        assert!(directory.entries().is_empty());
    }

    #[test]
    fn a_failure_after_publication_is_reported_as_not_durable() {
        let directory = Directory::new("not-durable");
        let destination = directory.0.join("target.yaml");
        let late = publication(&directory, &destination, PublishPolicy::Replace)
            .publish(b"published", accept, |step, _, _| {
                if step == PublishStep::SyncDirectory {
                    Err("sync failed")
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert!(matches!(
            late,
            PublishError::NotDurable(PublishFailure::Caller("sync failed"))
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"published");
        assert_eq!(directory.entries(), ["target.yaml"]);
    }

    #[test]
    fn a_vanished_temporary_file_fails_create_new_publication_cleanly() {
        let directory = Directory::new("vanished");
        let destination = directory.0.join("target.yaml");
        let failed = publication(&directory, &destination, PublishPolicy::CreateNew)
            .publish(b"bytes", accept, |step, temporary, _| {
                if step == PublishStep::Publish {
                    fs::remove_file(temporary).unwrap();
                }
                Ok::<(), &str>(())
            })
            .unwrap_err();
        assert!(matches!(
            failed,
            PublishError::NotPublished(PublishFailure::Io {
                step: PublishStep::Publish,
                ..
            })
        ));
        assert!(directory.entries().is_empty());
    }
}
