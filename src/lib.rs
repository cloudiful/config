mod env;
mod file;
mod paths;
mod secret;

use std::io;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReadOptions<'a> {
    pub env_prefix: Option<&'a str>,
}

impl<'a> ReadOptions<'a> {
    pub const fn with_env_prefix(env_prefix: &'a str) -> Self {
        Self {
            env_prefix: Some(env_prefix),
        }
    }
}

/// Save config to the platform-default `config.toml` for `app_name`.
///
/// On macOS, `stock` resolves to
/// `~/Library/Application Support/stock/config.toml`.
///
/// ```rust,no_run
/// use cloudiful_config::save;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// save("stock", AppConfig { port: 8080 }).unwrap();
/// ```
pub fn save<T>(app_name: &str, config: T) -> io::Result<()>
where
    T: serde::Serialize,
{
    let path = paths::default_config_path(app_name)?;
    file::write_config(&path, &config, file::FileType::TOML)
}

/// Read config from the platform-default `config.toml` for `app_name`,
/// creating the file from `T::default()` when it does not already exist.
///
/// Use [`ReadOptions`] to apply environment variable overrides after the file
/// is loaded.
///
/// ```rust,no_run
/// use cloudiful_config::{ReadOptions, read};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Default, Deserialize, Serialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// let _config: AppConfig = read("stock", Some(ReadOptions::with_env_prefix("STOCK_"))).unwrap();
/// ```
pub fn read<T>(app_name: &str, options: Option<ReadOptions<'_>>) -> Result<T, io::Error>
where
    T: serde::de::DeserializeOwned + Default + serde::Serialize,
{
    let path = paths::default_config_path(app_name)?;
    let mut config_value = if !path.is_file() {
        let default_config = T::default();
        file::write_config(&path, &default_config, file::FileType::TOML)?;
        serde_json::to_value(default_config).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "failed to serialize default config before applying overrides for {}: {e}",
                    path.display()
                ),
            )
        })?
    } else {
        file::read_config_value(&path)?
    };

    if let Some(prefix) = options.and_then(|options| options.env_prefix) {
        config_value = env::apply_env_overrides(config_value, prefix)?;
    }

    secret::resolve_secret_refs(&mut config_value)?;

    serde_json::from_value(config_value).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to deserialize config {} into requested type: {e}",
                path.display()
            ),
        )
    })
}

#[cfg(test)]
mod tests;
