use std::ffi::OsString;
use std::io::{self, ErrorKind};
use std::path::Component;
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";

pub(crate) fn default_config_path(app_name: &str) -> io::Result<PathBuf> {
    let app_name = Path::new(app_name);
    validate_app_name(app_name)?;
    Ok(config_root_from(|key| std::env::var_os(key))?
        .join(app_name)
        .join(DEFAULT_CONFIG_FILE_NAME))
}

fn validate_app_name(app_name: &Path) -> io::Result<()> {
    if app_name.as_os_str().is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "app name must not be empty",
        ));
    }

    if app_name.is_absolute() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("app name must be relative, got {}", app_name.display()),
        ));
    }

    match app_name.components().next() {
        Some(Component::Normal(_)) if app_name.components().count() == 1 => Ok(()),
        _ => Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "app name must be a single path component, got {}",
                app_name.display()
            ),
        )),
    }
}

fn env_path_with(get_env: impl Fn(&str) -> Option<OsString>, key: &str) -> Option<PathBuf> {
    get_env(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(windows)]
fn config_root_from(get_env: impl Fn(&str) -> Option<OsString>) -> io::Result<PathBuf> {
    env_path_with(&get_env, "APPDATA")
        .or_else(|| {
            env_path_with(&get_env, "USERPROFILE")
                .map(|path| path.join("AppData").join("Roaming"))
        })
        .or_else(|| match (get_env("HOMEDRIVE"), get_env("HOMEPATH")) {
            (Some(drive), Some(path)) if !drive.is_empty() && !path.is_empty() => {
                Some(PathBuf::from(drive).join(path))
            }
            _ => None,
        })
        .map(|path| {
            if path.ends_with("Roaming") {
                path
            } else {
                path.join("AppData").join("Roaming")
            }
        })
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                "failed to resolve Windows config directory from APPDATA, USERPROFILE, or HOMEDRIVE/HOMEPATH",
            )
        })
}

#[cfg(target_os = "macos")]
fn config_root_from(get_env: impl Fn(&str) -> Option<OsString>) -> io::Result<PathBuf> {
    env_path_with(get_env, "HOME")
        .map(|path| path.join("Library").join("Application Support"))
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                "failed to resolve macOS config directory from HOME",
            )
        })
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn config_root_from(get_env: impl Fn(&str) -> Option<OsString>) -> io::Result<PathBuf> {
    env_path_with(&get_env, "XDG_CONFIG_HOME")
        .or_else(|| env_path_with(get_env, "HOME").map(|path| path.join(".config")))
        .ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                "failed to resolve config directory from XDG_CONFIG_HOME or HOME",
            )
        })
}

#[cfg(test)]
pub(crate) fn test_config_root_from(
    get_env: impl Fn(&str) -> Option<OsString>,
) -> io::Result<PathBuf> {
    config_root_from(get_env)
}
