mod common;

use cloudiful_config::{read, save};
use common::{Conf, SecretConf, expected_default_config_path, temp_dir, with_test_config_home, with_test_config_home_and_env_result, with_test_config_home_result};
use std::fs;
use std::io::ErrorKind;

#[test]
fn missing_config_is_written_as_toml() {
    let config_root = temp_dir();

    with_test_config_home(&config_root, || {
        let path = expected_default_config_path(&config_root, "stock");
        let conf: Conf = read("stock", None).unwrap();

        assert_eq!(conf.name, "hello");
        let content = fs::read_to_string(&path).unwrap();
        let written: Conf = toml::from_str(&content).unwrap();

        assert_eq!(written.hello, 32);
        assert!(path.exists());
    });
}

#[test]
fn save_writes_default_config_file() {
    let config_root = temp_dir();

    with_test_config_home(&config_root, || {
        let path = expected_default_config_path(&config_root, "stock");
        save("stock", Conf::default()).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let written: Conf = toml::from_str(&content).unwrap();

        assert_eq!(written, Conf::default());
    });
}

#[test]
fn invalid_existing_content_returns_error_instead_of_panicking() {
    let config_root = temp_dir();
    let path = expected_default_config_path(&config_root, "stock");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{\n  \"hello\": 32,\n  \"name\": \"bad\"\n}\n").unwrap();

    let err =
        with_test_config_home_result(&config_root, || read::<Conf>("stock", None)).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(err.to_string().contains("failed to parse TOML config"));
}

#[test]
fn read_existing_toml_is_returned() {
    let config_root = temp_dir();

    with_test_config_home(&config_root, || {
        save(
            "stock",
            Conf {
                hello: 7,
                name: "custom".to_string(),
                list: vec!["one".to_string()],
            },
        )
        .unwrap();

        let conf: Conf = read("stock", None).unwrap();

        assert_eq!(
            conf,
            Conf {
                hello: 7,
                name: "custom".to_string(),
                list: vec!["one".to_string()],
            }
        );
    });
}

#[test]
fn save_preserves_secret_reference_strings() {
    let config_root = temp_dir();

    with_test_config_home(&config_root, || {
        let config = SecretConf {
            database: common::SecretDatabaseConf {
                user: "app".to_string(),
                password: "secret://keyring?service=stock&user=db-prod".to_string(),
            },
            tokens: vec!["secret://keyring?service=stock&user=api-token".to_string()],
        };

        save("stock", &config).unwrap();

        let path = expected_default_config_path(&config_root, "stock");
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("secret://keyring?service=stock&user=db-prod"));
        assert!(content.contains("secret://keyring?service=stock&user=api-token"));
    });
}

#[test]
fn read_fails_when_secret_reference_cannot_be_resolved() {
    let config_root = temp_dir();

    with_test_config_home(&config_root, || {
        save(
            "stock",
            SecretConf {
                database: common::SecretDatabaseConf {
                    user: "app".to_string(),
                    password: "secret://keyring?service=stock&user=db-prod".to_string(),
                },
                tokens: vec!["plain-token".to_string()],
            },
        )
        .unwrap();
    });

    let err = with_test_config_home_result(&config_root, || read::<SecretConf>("stock", None)).unwrap_err();

    assert!(matches!(
        err.kind(),
        ErrorKind::Unsupported | ErrorKind::NotFound
    ));
    assert!(err.to_string().contains("database.password"));
}

#[test]
fn env_secret_override_is_applied_after_file_load() {
    let config_root = temp_dir();

    with_test_config_home(&config_root, || {
        save("stock", SecretConf::default()).unwrap();
    });

    let err = with_test_config_home_and_env_result(
        &config_root,
        &[(
            "APP_DATABASE__PASSWORD",
            Some("secret://keyring?service=stock&user=db-prod"),
        )],
        || {
            read::<SecretConf>(
                "stock",
                Some(cloudiful_config::ReadOptions::with_env_prefix("APP_")),
            )
        },
    )
    .unwrap_err();

    assert!(matches!(
        err.kind(),
        ErrorKind::Unsupported | ErrorKind::NotFound
    ));
    assert!(err.to_string().contains("database.password"));
}
