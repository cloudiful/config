mod common;

use cloudiful_config::{config_dir, config_path};
use common::with_env_changes;
use std::io::ErrorKind;
use std::path::PathBuf;

#[test]
fn config_path_rejects_absolute_paths() {
    let err = config_path("demo-app", std::env::temp_dir()).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("must be relative"));
}

#[test]
fn config_dir_rejects_empty_app_name() {
    let err = config_dir("").unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("must not be empty"));
}

#[cfg(all(not(windows), not(target_os = "macos")))]
#[test]
fn config_dir_prefers_xdg_config_home() {
    with_env_changes(
        &[
            ("XDG_CONFIG_HOME", Some("/tmp/xdg-config")),
            ("HOME", Some("/tmp/home")),
        ],
        || {
            let path = config_dir("demo-app").unwrap();

            assert_eq!(path, PathBuf::from("/tmp/xdg-config").join("demo-app"));
        },
    );
}

#[cfg(all(not(windows), not(target_os = "macos")))]
#[test]
fn config_dir_falls_back_to_home_dot_config() {
    with_env_changes(
        &[("XDG_CONFIG_HOME", None), ("HOME", Some("/tmp/home"))],
        || {
            let path = config_path("demo-app", "settings.toml").unwrap();

            assert_eq!(
                path,
                PathBuf::from("/tmp/home")
                    .join(".config")
                    .join("demo-app")
                    .join("settings.toml")
            );
        },
    );
}

#[cfg(target_os = "macos")]
#[test]
fn config_dir_uses_application_support_on_macos() {
    with_env_changes(&[("HOME", Some("/tmp/home"))], || {
        let path = config_path("demo-app", "settings.toml").unwrap();

        assert_eq!(
            path,
            PathBuf::from("/tmp/home")
                .join("Library")
                .join("Application Support")
                .join("demo-app")
                .join("settings.toml")
        );
    });
}

#[cfg(windows)]
#[test]
fn config_dir_uses_appdata_on_windows() {
    with_env_changes(
        &[
            ("APPDATA", Some(r"C:\Users\alice\AppData\Roaming")),
            ("USERPROFILE", None),
            ("HOMEDRIVE", None),
            ("HOMEPATH", None),
        ],
        || {
            let path = config_path("demo-app", "settings.toml").unwrap();

            assert_eq!(
                path,
                PathBuf::from(r"C:\Users\alice\AppData\Roaming")
                    .join("demo-app")
                    .join("settings.toml")
            );
        },
    );
}

#[cfg(windows)]
#[test]
fn config_dir_falls_back_to_userprofile_on_windows() {
    with_env_changes(
        &[
            ("APPDATA", None),
            ("USERPROFILE", Some(r"C:\Users\alice")),
            ("HOMEDRIVE", None),
            ("HOMEPATH", None),
        ],
        || {
            let path = config_dir("demo-app").unwrap();

            assert_eq!(
                path,
                PathBuf::from(r"C:\Users\alice")
                    .join("AppData")
                    .join("Roaming")
                    .join("demo-app")
            );
        },
    );
}
