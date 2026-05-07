mod common;

use cloudiful_config::{read, save};
use common::{Conf, expected_default_config_path, temp_dir, with_test_config_home};
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

fn with_test_config_home_result<T>(
    config_root: &std::path::Path,
    test: impl FnOnce() -> Result<T, std::io::Error>,
) -> Result<T, std::io::Error> {
    let mut result = None;
    with_test_config_home(config_root, || {
        result = Some(test());
    });
    result.expect("test closure should run")
}
