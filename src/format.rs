use std::io::{self, ErrorKind};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigFormat {
    Toml,
    Json,
}

impl ConfigFormat {
    pub(crate) fn from_path(path: &Path) -> io::Result<Self> {
        match path.extension().and_then(|suffix| suffix.to_str()) {
            Some("toml") => Ok(Self::Toml),
            Some("json") | Some("jsonc") => Ok(Self::Json),
            _ => Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "config file type not supported for {} (expected .toml, .json, or .jsonc)",
                    path.display()
                ),
            )),
        }
    }

}

pub(crate) fn serialize_config<T>(
    config: &T,
    format: ConfigFormat,
    source: &str,
) -> io::Result<String>
where
    T: serde::Serialize + ?Sized,
{
    match format {
        ConfigFormat::Toml => toml::to_string_pretty(config).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("failed to serialize TOML config for {source}: {e}"),
            )
        }),
        ConfigFormat::Json => serde_json::to_string_pretty(config).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("failed to serialize JSON config for {source}: {e}"),
            )
        }),
    }
}

pub(crate) fn parse_config_value(
    content: &str,
    format: ConfigFormat,
    source: &str,
) -> io::Result<serde_json::Value> {
    match format {
        ConfigFormat::Toml => {
            let toml_value: toml::Value = toml::from_str(content).map_err(|e| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to parse TOML config {source}: {e}"),
                )
            })?;

            serde_json::to_value(toml_value).map_err(|e| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to convert TOML config {source} to JSON value: {e}"),
                )
            })
        }
        ConfigFormat::Json => serde_json::from_str(content).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("failed to parse JSON config {source}: {e}"),
            )
        }),
    }
}
