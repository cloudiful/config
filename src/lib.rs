mod env;
mod file;
mod format;
mod paths;
mod secret;
mod sql;

use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DotenvOptions<'a> {
    #[default]
    Enabled,
    Disabled,
    Path(&'a Path),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadOptions<'a> {
    pub env_prefix: Option<&'a str>,
    pub dotenv: DotenvOptions<'a>,
}

impl Default for ReadOptions<'_> {
    fn default() -> Self {
        Self {
            env_prefix: None,
            dotenv: DotenvOptions::Enabled,
        }
    }
}

impl<'a> ReadOptions<'a> {
    pub const fn with_env_prefix(env_prefix: &'a str) -> Self {
        Self {
            env_prefix: Some(env_prefix),
            dotenv: DotenvOptions::Enabled,
        }
    }

    pub const fn without_dotenv(mut self) -> Self {
        self.dotenv = DotenvOptions::Disabled;
        self
    }

    pub const fn with_dotenv_path(mut self, path: &'a Path) -> Self {
        self.dotenv = DotenvOptions::Path(path);
        self
    }

    pub const fn with_dotenv(mut self) -> Self {
        self.dotenv = DotenvOptions::Enabled;
        self
    }
}

fn load_dotenv(options: DotenvOptions<'_>) -> io::Result<()> {
    let result = match options {
        DotenvOptions::Enabled => dotenvy::from_path(".env").map(|_| ()),
        DotenvOptions::Disabled => return Ok(()),
        DotenvOptions::Path(path) => dotenvy::from_path(path).map(|_| ()),
    };

    match result {
        Ok(()) => Ok(()),
        Err(dotenvy::Error::Io(err))
            if matches!(options, DotenvOptions::Enabled)
                && err.kind() == io::ErrorKind::NotFound =>
        {
            Ok(())
        }
        Err(err) => {
            let source = match options {
                DotenvOptions::Enabled => ".env".to_string(),
                DotenvOptions::Disabled => unreachable!("disabled dotenv already returned"),
                DotenvOptions::Path(path) => path.display().to_string(),
            };

            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to load dotenv file {source}: {err}"),
            ))
        }
    }
}

pub trait ConfigSource {
    fn source_name(&self) -> String;
    fn read_value(&mut self) -> io::Result<Option<serde_json::Value>>;
    fn write_config<T>(&mut self, config: &T) -> io::Result<()>
    where
        T: serde::Serialize;
}

impl<T> ConfigSource for &mut T
where
    T: ConfigSource + ?Sized,
{
    fn source_name(&self) -> String {
        (**self).source_name()
    }

    fn read_value(&mut self) -> io::Result<Option<serde_json::Value>> {
        (**self).read_value()
    }

    fn write_config<S>(&mut self, config: &S) -> io::Result<()>
    where
        S: serde::Serialize,
    {
        (**self).write_config(config)
    }
}

impl ConfigSource for &str {
    fn source_name(&self) -> String {
        match paths::default_config_path(self) {
            Ok(path) => path.display().to_string(),
            Err(_) => (*self).to_string(),
        }
    }

    fn read_value(&mut self) -> io::Result<Option<serde_json::Value>> {
        let path = paths::default_config_path(self)?;
        if path.is_file() {
            file::read_config_value(&path).map(Some)
        } else {
            Ok(None)
        }
    }

    fn write_config<T>(&mut self, config: &T) -> io::Result<()>
    where
        T: serde::Serialize,
    {
        let path = paths::default_config_path(self)?;
        file::write_config(&path, config, file::FileType::TOML)
    }
}

pub fn save<T>(mut source: impl ConfigSource, config: T) -> io::Result<()>
where
    T: serde::Serialize,
{
    source.write_config(&config)
}

pub fn read<T>(
    mut source: impl ConfigSource,
    options: Option<ReadOptions<'_>>,
) -> Result<T, io::Error>
where
    T: serde::de::DeserializeOwned + Default + serde::Serialize,
{
    let options = options.unwrap_or_default();
    load_dotenv(options.dotenv)?;

    let source_name = source.source_name();
    let config_value = match source.read_value()? {
        Some(value) => value,
        None => {
            let default_config = T::default();
            let default_value = serde_json::to_value(&default_config).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "failed to serialize default config before applying overrides for {source_name}: {e}"
                    ),
                )
            })?;
            source.write_config(&default_config)?;
            default_value
        }
    };

    process_config_value(config_value, options.env_prefix, &source_name)
}

fn process_config_value<T>(
    mut config_value: serde_json::Value,
    env_prefix: Option<&str>,
    source: &str,
) -> Result<T, io::Error>
where
    T: serde::de::DeserializeOwned,
{
    if let Some(prefix) = env_prefix {
        config_value = env::apply_env_overrides(config_value, prefix)?;
    }

    secret::resolve_secret_refs(&mut config_value)?;

    serde_json::from_value(config_value).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to deserialize config {source} into requested type: {e}"),
        )
    })
}

pub use sql::postgres_store;
pub use sql::postgres_store_with_table;
pub use sql::DEFAULT_CONFIG_TABLE;
pub use sql::PostgresConfigStore;

#[cfg(test)]
mod tests;
