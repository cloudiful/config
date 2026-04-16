mod common;

use cloudiful_config::{ConfigSource, read};
use common::{EnvConf, NestedConf, temp_path, with_env_vars};
use std::fs;
use std::path::Path;

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
