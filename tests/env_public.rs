mod common;

use cloudiful_config::{ReadOptions, read, save};
use common::{EnvConf, NestedConf, temp_dir, with_test_config_home, with_test_config_home_and_env};

#[test]
fn env_prefix_overrides_defaults_when_file_is_missing() {
    let home = temp_dir();

    with_test_config_home_and_env(
        &home,
        &[
            ("APP_HOST", Some("svc")),
            ("APP_PORT", Some("9090")),
            ("APP_DEBUG", Some("true")),
            ("APP_TAGS", Some("[\"api\",\"edge\"]")),
            ("APP_DATABASE__URL", Some("postgres://db/service")),
            ("APP_DATABASE__POOL_SIZE", None),
        ],
        || {
            let conf: EnvConf = read("stock", Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

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
    let home = temp_dir();

    with_test_config_home_and_env(
        &home,
        &[
            ("APP_HOST", Some("{not-json}")),
            ("APP_DATABASE__POOL_SIZE", None),
        ],
        || {
            let conf: EnvConf = read("stock", Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

            assert_eq!(conf.host, "{not-json}");
        },
    );
}

#[test]
fn env_empty_segments_are_ignored() {
    let home = temp_dir();

    with_test_config_home_and_env(
        &home,
        &[
            ("APP_", Some("\"ignored\"")),
            ("APP____", Some("\"also ignored\"")),
            ("APP_DATABASE__POOL_SIZE", None),
        ],
        || {
            let conf: EnvConf = read("stock", Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

            assert_eq!(conf, EnvConf::default());
        },
    );
}

#[test]
fn env_values_override_file_values() {
    let home = temp_dir();

    with_test_config_home(&home, || {
        save(
            "stock",
            EnvConf {
                host: "from-file".to_string(),
                port: 8081,
                debug: false,
                tags: vec!["file".to_string()],
                database: NestedConf {
                    url: "postgres://db/file".to_string(),
                    pool_size: 10,
                },
            },
        )
        .unwrap();
    });

    with_test_config_home_and_env(
        &home,
        &[
            ("APP_PORT", Some("9090")),
            ("APP_DATABASE__POOL_SIZE", Some("20")),
            ("APP_TAGS", Some("[\"env\"]")),
        ],
        || {
            let conf: EnvConf = read("stock", Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

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
