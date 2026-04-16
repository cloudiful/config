use std::ffi::OsString;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// Resolve the per-user config directory for `app_name` on the current OS.
pub fn config_dir(app_name: &str) -> io::Result<PathBuf> {
    let app_name = Path::new(app_name);
    validate_relative_path(app_name, "app name")?;
    Ok(config_root()?.join(app_name))
}

/// Resolve a config file path relative to the per-user config directory.
pub fn config_path<P>(app_name: &str, relative_path: P) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    let relative_path = relative_path.as_ref();
    validate_relative_path(relative_path, "config path")?;
    Ok(config_dir(app_name)?.join(relative_path))
}

fn validate_relative_path(path: &Path, label: &str) -> io::Result<()> {
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not be empty"),
        ));
    }

    if path.is_absolute() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must be relative, got {}", path.display()),
        ));
    }

    Ok(())
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

fn config_root() -> io::Result<PathBuf> {
    config_root_from(|key| std::env::var_os(key))
}

#[cfg(test)]
pub(crate) fn test_config_root_from(
    get_env: impl Fn(&str) -> Option<OsString>,
) -> io::Result<PathBuf> {
    config_root_from(get_env)
}
