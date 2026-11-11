//! Device-independent stream recovery, kept out of the real-time audio callback.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy_platform::time::Instant;

use rodio::{
    DeviceSinkBuilder, MixerDeviceSink,
    cpal::{
        StreamError,
        traits::{DeviceTrait, HostTrait},
    },
};

const FIRST_RETRY: Duration = Duration::from_secs(1);
const MAX_RETRY: Duration = Duration::from_secs(30);
const HEALTHY_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Default)]
pub(crate) struct StreamErrors(Arc<Mutex<Option<String>>>);

impl StreamErrors {
    fn report(&self, error: StreamError) {
        // CPAL already prepares/restarts the PCM after an underrun. Reopening it here
        // would interrupt that recovery and restart otherwise healthy music.
        if matches!(error, StreamError::BufferUnderrun) {
            return;
        }
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if pending.is_none() {
            *pending = Some(error.to_string());
        }
    }

    fn take(&self) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }
}

pub(crate) enum OutputChange {
    None,
    Lost(String),
    Unavailable(String),
    Restored,
}

pub(crate) struct RecoveringOutput<S> {
    pub(crate) stream: Option<S>,
    errors: StreamErrors,
    next_retry: Instant,
    retry_delay: Duration,
    opened_at: Option<Instant>,
}

impl<S> RecoveringOutput<S> {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            stream: None,
            errors: StreamErrors::default(),
            next_retry: now,
            retry_delay: FIRST_RETRY,
            opened_at: None,
        }
    }

    pub(crate) fn update(
        &mut self,
        now: Instant,
        open: impl FnOnce(StreamErrors) -> Result<S, String>,
    ) -> OutputChange {
        if self
            .opened_at
            .is_some_and(|opened| now.duration_since(opened) >= HEALTHY_INTERVAL)
        {
            self.retry_delay = FIRST_RETRY;
        }

        if let Some(error) = self.errors.take() {
            if self.stream.take().is_some() {
                // Dropping the stream stops ALSA's worker before scheduling a retry.
                self.opened_at = None;
                self.schedule_retry(now);
                return OutputChange::Lost(error);
            }
        }

        if self.stream.is_some() || now < self.next_retry {
            return OutputChange::None;
        }

        // Each attempt gets a new mailbox: late callbacks from a failed attempt
        // must not invalidate the next stream.
        self.errors = StreamErrors::default();
        match open(self.errors.clone()) {
            Ok(stream) => {
                self.stream = Some(stream);
                self.opened_at = Some(now);
                OutputChange::Restored
            }
            Err(error) => {
                self.schedule_retry(now);
                OutputChange::Unavailable(error)
            }
        }
    }

    fn schedule_retry(&mut self, now: Instant) {
        self.next_retry = now + self.retry_delay;
        self.retry_delay = (self.retry_delay * 2).min(MAX_RETRY);
    }
}

pub(crate) fn open_output(errors: StreamErrors) -> Result<MixerDeviceSink, String> {
    let open = |builder: DeviceSinkBuilder| {
        let errors = errors.clone();
        builder
            .with_error_callback(move |error| errors.report(error))
            .open_sink_or_fallback()
    };

    // Keep Rodio's default-device and alternate-device/configuration fallbacks,
    // installing our callback on every attempt instead of the unbounded logger.
    let result = DeviceSinkBuilder::from_default_device().and_then(&open);
    let result = result
        .map_err(|error| error.to_string())
        .or_else(|original_error| {
            let devices = rodio::cpal::default_host()
                .output_devices()
                .map_err(|_| original_error.clone())?;
            devices
                .filter(|device| {
                    device
                        .description()
                        .map(|description| {
                            description.driver().is_some_and(|driver| driver != "null")
                        })
                        .unwrap_or(false)
                })
                .filter_map(|device| DeviceSinkBuilder::from_device(device).ok())
                .find_map(|builder| open(builder).ok())
                .ok_or(original_error)
        });

    result.map(|mut stream| {
        stream.log_on_drop(false);
        stream
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct FakeStream(Arc<AtomicBool>);

    impl Drop for FakeStream {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn backend_error() -> StreamError {
        rodio::cpal::BackendSpecificError {
            description: "`alsa::poll()` returned POLLERR".into(),
        }
        .into()
    }

    #[test]
    fn repeated_poll_errors_drop_the_failed_stream_and_recover_after_a_delay() {
        let now = Instant::now();
        let dropped = Arc::new(AtomicBool::new(false));
        let mut output = RecoveringOutput::new(now);
        output.update(now, |_| Ok(FakeStream(dropped.clone())));
        for _ in 0..10_000 {
            output.errors.report(backend_error());
        }
        assert!(matches!(
            output.update(now, |_| panic!("retry too soon")),
            OutputChange::Lost(_)
        ));
        assert!(dropped.load(Ordering::SeqCst));
        assert!(output.stream.is_none());
        assert!(matches!(
            output.update(now, |_| panic!("retry too soon")),
            OutputChange::None
        ));
        assert!(matches!(
            output.update(now + FIRST_RETRY, |_| Ok(FakeStream(dropped.clone()))),
            OutputChange::Restored
        ));
        assert!(output.stream.is_some());
    }

    #[test]
    fn unavailable_devices_use_bounded_exponential_backoff() {
        let mut now = Instant::now();
        let mut output = RecoveringOutput::<()>::new(now);
        for seconds in [1, 2, 4, 8, 16, 30, 30] {
            assert!(matches!(
                output.update(now, |_| Err("no device".into())),
                OutputChange::Unavailable(_)
            ));
            assert_eq!(
                output.next_retry.duration_since(now),
                Duration::from_secs(seconds)
            );
            now = output.next_retry;
            assert!(matches!(
                output.update(now - Duration::from_millis(1), |_| panic!("retry too soon")),
                OutputChange::None
            ));
        }
    }

    #[test]
    fn a_quickly_failing_reopened_stream_does_not_reset_backoff() {
        let now = Instant::now();
        let mut output = RecoveringOutput::new(now);
        output.update(now, |_| Ok(()));
        output.errors.report(backend_error());
        output.update(now, |_| unreachable!());
        output.update(now + FIRST_RETRY, |_| Ok(()));
        output.errors.report(backend_error());
        output.update(now + FIRST_RETRY, |_| unreachable!());
        assert_eq!(
            output.next_retry.duration_since(now + FIRST_RETRY),
            Duration::from_secs(2)
        );

        let reopened = output.next_retry;
        output.update(reopened, |_| Ok(()));
        output.errors.report(backend_error());
        output.update(reopened + HEALTHY_INTERVAL, |_| unreachable!());
        assert_eq!(
            output
                .next_retry
                .duration_since(reopened + HEALTHY_INTERVAL),
            FIRST_RETRY
        );
    }

    #[test]
    fn underruns_recover_in_cpal_without_reopening_or_restarting_music() {
        let now = Instant::now();
        let mut output = RecoveringOutput::new(now);
        output.update(now, |_| Ok(()));
        output.errors.report(StreamError::BufferUnderrun);
        assert!(matches!(
            output.update(now, |_| panic!("healthy stream")),
            OutputChange::None
        ));
        assert!(output.stream.is_some());
    }

    #[test]
    fn old_callbacks_cannot_invalidate_a_replacement_stream() {
        let now = Instant::now();
        let mut output = RecoveringOutput::new(now);
        output.update(now, |_| Ok(()));
        let old_callback = output.errors.clone();
        old_callback.report(backend_error());
        output.update(now, |_| unreachable!());
        output.update(now + FIRST_RETRY, |_| Ok(()));
        old_callback.report(backend_error());
        assert!(matches!(
            output.update(now + FIRST_RETRY, |_| panic!("healthy stream")),
            OutputChange::None
        ));
    }
}
