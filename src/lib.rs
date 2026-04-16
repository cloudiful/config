mod env;
mod file;
mod paths;

use std::io;
use std::path::Path;

pub use file::FileType;
pub use paths::{config_dir, config_path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSource<P, S> {
    File(P),
    Env { prefix: S },
    FileWithEnv { path: P, prefix: S },
}

/// Save the current config to a TOML or JSON file.
///
/// The `file_type` must match the file extension inferred from `path`.
/// `.jsonc` paths are written as standard JSON and therefore require
/// [`FileType::JSON`].
///
/// ```rust,no_run
/// use cloudiful_config::{FileType, save};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// let path = std::env::temp_dir().join("config-crate-save-example.toml");
/// save(&path, AppConfig { port: 8080 }, FileType::TOML).unwrap();
/// ```
pub fn save<P, T>(path: P, config: T, file_type: FileType) -> io::Result<()>
where
    P: AsRef<Path>,
    T: serde::Serialize,
{
    file::write_config(path.as_ref(), &config, file_type)
}

/// Save the current config by inferring the format from the file extension.
///
/// `.toml` writes TOML. `.json` and `.jsonc` write standard JSON.
///
/// ```rust,no_run
/// use cloudiful_config::save_inferred;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct AppConfig {
///     debug: bool,
/// }
///
/// let path = std::env::temp_dir().join("config-crate-save-inferred.jsonc");
/// save_inferred(&path, AppConfig { debug: true }).unwrap();
/// ```
pub fn save_inferred<P, T>(path: P, config: T) -> io::Result<()>
where
    P: AsRef<Path>,
    T: serde::Serialize,
{
    file::write_config_inferred(path.as_ref(), &config)
}

/// Read config from an existing TOML, JSON, or JSONC file.
///
/// `.jsonc` accepts both `//` line comments and `/* ... */` block comments.
///
/// ```rust,no_run
/// use cloudiful_config::read_existing;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// let path = std::env::temp_dir().join("config-crate-read-existing.toml");
/// let _config: AppConfig = read_existing(&path).unwrap();
/// ```
pub fn read_existing<P, T>(path: P) -> Result<T, io::Error>
where
    P: AsRef<Path>,
    T: serde::de::DeserializeOwned,
{
    file::read_config(path.as_ref())
}

/// Read config from a TOML, JSON, or JSONC file, creating the file with
/// `T::default()` when it does not already exist.
///
/// ```rust,no_run
/// use cloudiful_config::read_or_create_default;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Default, Deserialize, Serialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// let path = std::env::temp_dir().join("config-crate-read-or-create.toml");
/// let _config: AppConfig = read_or_create_default(&path).unwrap();
/// ```
pub fn read_or_create_default<P, T>(path: P) -> Result<T, io::Error>
where
    P: AsRef<Path>,
    T: serde::de::DeserializeOwned + Default + serde::Serialize,
{
    let path = path.as_ref();
    let file_type = file::infer_file_type(path)?;

    if !path.is_file() {
        let default_config = T::default();
        file::write_config(path, &default_config, file_type)?;
        return Ok(default_config);
    }

    file::read_config(path)
}

/// Read config from an explicit source.
///
/// `ConfigSource::File` keeps the historical behavior of creating the file from
/// `T::default()` when it does not exist.
///
/// Environment variables map to fields by lowercasing the suffix after
/// `prefix`. Use `__` for nested fields and provide JSON literals for typed
/// values such as arrays and booleans.
///
/// ```rust,no_run
/// use cloudiful_config::{ConfigSource, read};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Default, Deserialize, Serialize)]
/// struct DatabaseConfig {
///     url: String,
/// }
///
/// #[derive(Default, Deserialize, Serialize)]
/// struct AppConfig {
///     database: DatabaseConfig,
/// }
///
/// let path = std::env::temp_dir().join("config-crate-read-with-env.toml");
/// unsafe {
///     std::env::set_var("APP_DATABASE__URL", "\"postgres://db/service\"");
/// }
/// let _config: AppConfig = read(ConfigSource::FileWithEnv {
///     path: &path,
///     prefix: "APP_",
/// })
/// .unwrap();
/// unsafe {
///     std::env::remove_var("APP_DATABASE__URL");
/// }
/// ```
pub fn read<P, S, T>(source: ConfigSource<P, S>) -> Result<T, io::Error>
where
    P: AsRef<Path>,
    S: AsRef<str>,
    T: serde::de::DeserializeOwned + Default + serde::Serialize,
{
    match source {
        ConfigSource::File(path) => read_or_create_default(path),
        ConfigSource::Env { prefix } => env::apply_env_overrides(T::default(), prefix.as_ref()),
        ConfigSource::FileWithEnv { path, prefix } => {
            let config = read_or_create_default(path)?;
            env::apply_env_overrides(config, prefix.as_ref())
        }
    }
}

#[cfg(test)]
mod tests;
