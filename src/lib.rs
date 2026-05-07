mod env;
mod file;
mod paths;

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
    let config = if !path.is_file() {
        let default_config = T::default();
        file::write_config(&path, &default_config, file::FileType::TOML)?;
        default_config
    } else {
        file::read_config(&path)?
    };

    match options.and_then(|options| options.env_prefix) {
        Some(prefix) => env::apply_env_overrides(config, prefix),
        None => Ok(config),
    }
}

#[cfg(test)]
mod tests;
