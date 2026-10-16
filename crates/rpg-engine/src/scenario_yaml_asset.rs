//! The shared Bevy loader for typed scenario YAML documents.
//!
//! Every YAML asset reads its bytes, requires UTF-8, and parses through [`scenario_yaml`]. Each
//! registration names its subject, so a failure still says which document kind owns it.

use std::{error::Error, fmt, io, str::Utf8Error};

use bevy::{
    asset::{Asset, AssetLoadError, AssetLoader, LoadContext, io::AssetReaderError, io::Reader},
    reflect::TypePath,
};
use serde::de::DeserializeOwned;

use crate::scenario_yaml::{self, ScenarioYamlError};

/// Loads one typed YAML asset; register it with `App::register_asset_loader`.
#[derive(TypePath)]
pub(crate) struct ScenarioYamlLoader<A: Asset> {
    subject: &'static str,
    parse: fn(&str) -> Result<A, ScenarioYamlError>,
}

impl<A: Asset + DeserializeOwned> ScenarioYamlLoader<A> {
    pub(crate) fn new(subject: &'static str) -> Self {
        Self::with_parser(subject, scenario_yaml::from_str::<A>)
    }
}

impl<A: Asset> ScenarioYamlLoader<A> {
    /// A loader for a document whose shape needs more than one `Deserialize` call.
    pub(crate) fn with_parser(
        subject: &'static str,
        parse: fn(&str) -> Result<A, ScenarioYamlError>,
    ) -> Self {
        Self { subject, parse }
    }
}

impl<A: Asset> AssetLoader for ScenarioYamlLoader<A> {
    type Asset = A;
    type Settings = ();
    type Error = ScenarioYamlAssetError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<A, Self::Error> {
        let fail = |cause| ScenarioYamlAssetError {
            subject: self.subject,
            cause,
        };
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|error| fail(YamlAssetCause::Io(error)))?;
        let document =
            std::str::from_utf8(&bytes).map_err(|error| fail(YamlAssetCause::Utf8(error)))?;
        (self.parse)(document).map_err(|error| fail(YamlAssetCause::Yaml(error)))
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[derive(Debug)]
pub(crate) struct ScenarioYamlAssetError {
    subject: &'static str,
    cause: YamlAssetCause,
}

#[derive(Debug)]
enum YamlAssetCause {
    Io(io::Error),
    Utf8(Utf8Error),
    Yaml(ScenarioYamlError),
}

impl fmt::Display for ScenarioYamlAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let subject = self.subject;
        match &self.cause {
            YamlAssetCause::Io(error) => write!(formatter, "{subject} read failed: {error}"),
            YamlAssetCause::Utf8(error) => write!(formatter, "{subject} is not UTF-8: {error}"),
            YamlAssetCause::Yaml(error) => write!(formatter, "{subject} YAML is invalid: {error}"),
        }
    }
}

impl Error for ScenarioYamlAssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.cause {
            YamlAssetCause::Io(error) => Some(error),
            YamlAssetCause::Utf8(error) => Some(error),
            YamlAssetCause::Yaml(error) => Some(error),
        }
    }
}

/// A load failure safe to show the player: reader errors report only their kind, never a
/// host path, and loader errors use the YAML loader's own message.
pub(crate) fn safe_asset_load_cause(error: &AssetLoadError, subject: &str) -> String {
    match error {
        AssetLoadError::AssetReaderError(AssetReaderError::NotFound(_)) => {
            format!("{subject} was not found")
        }
        AssetLoadError::AssetReaderError(AssetReaderError::Io(error)) => {
            format!("{subject} I/O failed ({:?})", error.kind())
        }
        AssetLoadError::AssetReaderError(AssetReaderError::HttpError(status)) => {
            format!("{subject} request returned HTTP status {status}")
        }
        AssetLoadError::AssetLoaderError(error) => error
            .error()
            .downcast_ref::<ScenarioYamlAssetError>()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{subject} loader failed")),
        _ => format!("{subject} loading failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(cause: YamlAssetCause) -> String {
        ScenarioYamlAssetError {
            subject: "quest catalog",
            cause,
        }
        .to_string()
    }

    #[test]
    fn every_failure_names_its_registered_subject() {
        let io = io::Error::new(io::ErrorKind::UnexpectedEof, "short read");
        assert_eq!(
            message(YamlAssetCause::Io(io)),
            "quest catalog read failed: short read"
        );
        let bytes = vec![0xff];
        let utf8 = std::str::from_utf8(&bytes).unwrap_err();
        assert!(message(YamlAssetCause::Utf8(utf8)).starts_with("quest catalog is not UTF-8: "));
        let yaml = scenario_yaml::from_str::<u32>("[").unwrap_err();
        assert!(message(YamlAssetCause::Yaml(yaml)).starts_with("quest catalog YAML is invalid: "));
    }
}
