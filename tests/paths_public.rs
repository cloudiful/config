mod common;

use cloudiful_config::{read, save};
use common::{Conf, temp_dir, with_env_changes};
use std::fs;
use std::io::ErrorKind;

#[test]
fn read_rejects_empty_app_name() {
    let err = read::<Conf>("", None).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("must not be empty"));
}

#[test]
fn save_rejects_nested_app_name() {
    let err = save("demo-app/nested", Conf::default()).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("single path component"));
}

#[cfg(all(not(windows), not(target_os = "macos")))]
#[test]
fn read_prefers_xdg_config_home() {
    let xdg_config_home = temp_dir();
    let xdg_config_home_str = xdg_config_home.to_string_lossy().into_owned();

    with_env_changes(
        &[
            ("XDG_CONFIG_HOME", Some(xdg_config_home_str.as_str())),
            ("HOME", None),
        ],
        || {
            let app_name = "stock";
            let config: Conf = read(app_name, None).unwrap();
            let path = xdg_config_home.join(app_name).join("config.toml");

            assert_eq!(config, Conf::default());
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                toml::to_string_pretty(&config).unwrap()
            );
        },
    );
}

#[cfg(target_os = "macos")]
#[test]
fn save_uses_application_support_on_macos() {
    let home = temp_dir();
    let home_str = home.to_string_lossy().into_owned();

    with_env_changes(&[("HOME", Some(home_str.as_str()))], || {
        save("stock", Conf::default()).unwrap();

        let path = home
            .join("Library")
            .join("Application Support")
            .join("stock")
            .join("config.toml");

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            toml::to_string_pretty(&Conf::default()).unwrap()
        );
    });
}

#[cfg(windows)]
#[test]
fn save_uses_appdata_on_windows() {
    let appdata = temp_dir();
    let appdata_str = appdata.to_string_lossy().into_owned();

    with_env_changes(
        &[
            ("APPDATA", Some(appdata_str.as_str())),
            ("USERPROFILE", None),
            ("HOMEDRIVE", None),
            ("HOMEPATH", None),
        ],
        || {
            save("stock", Conf::default()).unwrap();

            let path = appdata.join("stock").join("config.toml");

            assert_eq!(
                fs::read_to_string(path).unwrap(),
                toml::to_string_pretty(&Conf::default()).unwrap()
            );
        },
    );
}

#[cfg(all(not(windows), not(target_os = "macos")))]
#[test]
fn read_falls_back_to_home_dot_config() {
    let home = temp_dir();
    let home_str = home.to_string_lossy().into_owned();

    with_env_changes(
        &[("XDG_CONFIG_HOME", None), ("HOME", Some(home_str.as_str()))],
        || {
            let app_name = "stock";
            let _: Conf = read(app_name, None).unwrap();
            let path = home.join(".config").join(app_name).join("config.toml");

            assert!(path.exists());
        },
    );
}
