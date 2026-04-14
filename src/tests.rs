use super::{ConfigSource, FileType, read, read_existing, save, save_inferred};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Conf {
    hello: i32,
    name: String,
    list: Vec<String>,
}

impl Default for Conf {
    fn default() -> Self {
        Self {
            hello: 32,
            name: "hello".to_string(),
            list: vec!["test1".to_string(), "test2".to_string()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct JsoncStringConf {
    url: String,
    note: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct NestedConf {
    url: String,
    pool_size: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct EnvConf {
    host: String,
    port: u16,
    debug: bool,
    tags: Vec<String>,
    database: NestedConf,
}

impl Default for EnvConf {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            debug: false,
            tags: vec!["default".to_string()],
            database: NestedConf {
                url: "sqlite:///tmp/default.db".to_string(),
                pool_size: 5,
            },
        }
    }
}

fn temp_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "config-crate-tests-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_path(file_name: &str) -> PathBuf {
    temp_dir().join(file_name)
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_env_vars(vars: &[(&str, &str)], test: impl FnOnce()) {
    let _guard = env_lock().lock().unwrap();
    let previous: Vec<(String, Option<OsString>)> = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var_os(key)))
        .collect();

    for (key, value) in vars {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    test();

    for (key, value) in previous {
        match value {
            Some(value) => unsafe {
                std::env::set_var(&key, value);
            },
            None => unsafe {
                std::env::remove_var(&key);
            },
        }
    }
}

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
fn env_only_config_can_use_defaults_as_base() {
    with_env_vars(
        &[
            ("APP_HOST", "svc"),
            ("APP_PORT", "9090"),
            ("APP_DEBUG", "true"),
            ("APP_TAGS", "[\"api\",\"edge\"]"),
            ("APP_DATABASE__URL", "postgres://db/service"),
        ],
        || {
            let conf: EnvConf = read(ConfigSource::<&Path, _>::Env { prefix: "APP_" }).unwrap();

            assert_eq!(
                conf,
                EnvConf {
                    host: "svc".to_string(),
                    port: 9090,
                    debug: true,
                    tags: vec!["api".to_string(), "edge".to_string()],
                    database: NestedConf {
                        url: "postgres://db/service".to_string(),
                        pool_size: 5,
                    },
                }
            );
        },
    );
}

#[test]
fn env_invalid_json_falls_back_to_plain_string() {
    with_env_vars(&[("APP_HOST", "{not-json}")], || {
        let conf: EnvConf = read(ConfigSource::<&Path, _>::Env { prefix: "APP_" }).unwrap();

        assert_eq!(conf.host, "{not-json}");
    });
}

#[test]
fn env_empty_segments_are_ignored() {
    with_env_vars(
        &[("APP_", "\"ignored\""), ("APP____", "\"also ignored\"")],
        || {
            let conf: EnvConf = read(ConfigSource::<&Path, _>::Env { prefix: "APP_" }).unwrap();

            assert_eq!(conf, EnvConf::default());
        },
    );
}

#[test]
fn env_object_override_replaces_scalar_intermediate_node() {
    with_env_vars(&[("APP_DATABASE__URL", "\"postgres://override\"")], || {
        let value = serde_json::json!({
            "database": "from-default"
        });

        let merged: serde_json::Value = super::env::apply_env_overrides(value, "APP_").unwrap();

        assert_eq!(
            merged,
            serde_json::json!({
                "database": {
                    "url": "postgres://override"
                }
            })
        );
    });
}

#[test]
fn env_values_override_file_values() {
    let path = temp_path("config.toml");
    fs::write(
        &path,
        r#"
host = "from-file"
port = 8081
debug = false
tags = ["file"]

[database]
url = "postgres://db/file"
pool_size = 10
"#,
    )
    .unwrap();

    with_env_vars(
        &[
            ("APP_PORT", "9090"),
            ("APP_DATABASE__POOL_SIZE", "20"),
            ("APP_TAGS", "[\"env\"]"),
        ],
        || {
            let conf: EnvConf = read(ConfigSource::FileWithEnv {
                path: &path,
                prefix: "APP_",
            })
            .unwrap();

            assert_eq!(
                conf,
                EnvConf {
                    host: "from-file".to_string(),
                    port: 9090,
                    debug: false,
                    tags: vec!["env".to_string()],
                    database: NestedConf {
                        url: "postgres://db/file".to_string(),
                        pool_size: 20,
                    },
                }
            );
        },
    );
}

#[test]
fn file_source_variant_preserves_legacy_read_behavior() {
    let path = temp_path("legacy.toml");

    let conf: Conf = read(ConfigSource::<_, &str>::File(path.clone())).unwrap();

    assert_eq!(conf, Conf::default());
    assert!(path.exists());
}
