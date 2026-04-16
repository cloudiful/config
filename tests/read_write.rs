mod common;

use cloudiful_config::{ConfigSource, FileType, read, read_existing, save, save_inferred};
use common::{Conf, JsoncStringConf, temp_dir, temp_path};
use std::fs;
use std::io::ErrorKind;

#[test]
fn missing_toml_is_written_as_toml() {
    let path = temp_path("config.toml");

    let conf: Conf = read(ConfigSource::<_, &str>::File(path.clone())).unwrap();
    assert_eq!(conf.name, "hello");

    let content = fs::read_to_string(&path).unwrap();
    let written: Conf = toml::from_str(&content).unwrap();

    assert_eq!(written.hello, 32);
    assert!(serde_json::from_str::<serde_json::Value>(&content).is_err());
}

#[test]
fn missing_json_is_written_as_json() {
    let path = temp_path("config.json");

    let conf: Conf = read(ConfigSource::<_, &str>::File(path.clone())).unwrap();
    assert_eq!(conf.name, "hello");

    let content = fs::read_to_string(&path).unwrap();
    let written: Conf = serde_json::from_str(&content).unwrap();

    assert_eq!(written.hello, 32);
    assert!(content.trim_start().starts_with('{'));
}

#[test]
fn save_inferred_writes_json_for_jsonc_paths() {
    let path = temp_path("config.jsonc");

    save_inferred(&path, Conf::default()).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let written: Conf = serde_json::from_str(&content).unwrap();

    assert_eq!(written, Conf::default());
    assert!(content.trim_start().starts_with('{'));
}

#[test]
fn save_inferred_writes_toml_for_toml_paths() {
    let path = temp_path("config.toml");

    save_inferred(&path, Conf::default()).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let written: Conf = toml::from_str(&content).unwrap();

    assert_eq!(written, Conf::default());
}

#[test]
fn save_rejects_mismatched_file_type() {
    let path = temp_path("config.toml");

    let err = save(&path, Conf::default(), FileType::JSON).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains(path.to_string_lossy().as_ref()));
    assert!(err.to_string().contains("expects TOML"));
}

#[test]
fn save_inferred_rejects_unsupported_extension() {
    let path = temp_path("config.yaml");

    let err = save_inferred(&path, Conf::default()).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains(".toml, .json, or .jsonc"));
}

#[test]
fn invalid_content_returns_error_instead_of_panicking() {
    let path = temp_path("config.toml");
    fs::write(&path, "{\n  \"hello\": 32,\n  \"name\": \"bad\"\n}\n").unwrap();

    let err = read::<_, _, Conf>(ConfigSource::<_, &str>::File(path)).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(err.to_string().contains("failed to parse TOML config"));
}

#[test]
fn jsonc_comments_are_supported() {
    let path = temp_path("config.jsonc");
    fs::write(
        &path,
        r#"{
  // line comment
  "hello": 32,
  /* block comment */
  "name": "hello",
  "list": ["test1", "test2"]
}"#,
    )
    .unwrap();

    let conf: Conf = read_existing(path).unwrap();

    assert_eq!(conf.hello, 32);
    assert_eq!(conf.name, "hello");
}

#[test]
fn jsonc_comment_markers_inside_strings_are_preserved() {
    let path = temp_path("config.jsonc");
    fs::write(
        &path,
        r#"{
  "url": "https://example.com/api/*v1*/endpoint",
  "note": "literal // text"
}"#,
    )
    .unwrap();

    let conf: JsoncStringConf = read_existing(path).unwrap();

    assert_eq!(
        conf,
        JsoncStringConf {
            url: "https://example.com/api/*v1*/endpoint".to_string(),
            note: "literal // text".to_string(),
        }
    );
}

#[test]
fn read_existing_missing_file_returns_not_found() {
    let path = temp_path("missing.toml");

    let err = read_existing::<_, Conf>(&path).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert!(!path.exists());
}

#[test]
fn save_returns_error_for_directory_path() {
    let path = temp_dir();

    let err = save_inferred(&path, Conf::default()).unwrap_err();

    assert!(path.is_dir());
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(fs::read_dir(&path).unwrap().next().is_none());
}

#[test]
fn save_inferred_creates_missing_parent_directories() {
    let dir = temp_dir();
    let path = dir.join("nested").join("config.json");

    save_inferred(&path, Conf::default()).unwrap();

    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();
    let written: Conf = serde_json::from_str(&content).unwrap();
    assert_eq!(written, Conf::default());
}

#[test]
fn file_source_variant_preserves_legacy_read_behavior() {
    let path = temp_path("legacy.toml");

    let conf: Conf = read(ConfigSource::<_, &str>::File(path.clone())).unwrap();

    assert_eq!(conf, Conf::default());
    assert!(path.exists());
}
