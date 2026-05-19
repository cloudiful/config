mod common;

use cloudiful_config::{read, save, ReadOptions};
use common::{
    temp_dir, with_test_config_home, with_test_config_home_and_env,
    with_test_config_home_env_and_current_dir, EnvConf, NestedConf,
};

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

#[test]
fn dotenv_is_loaded_by_default_before_env_overrides() {
    let home = temp_dir();
    let app_dir = temp_dir();
    std::fs::write(
        app_dir.join(".env"),
        "APP_HOST=from-dotenv\nAPP_PORT=9091\nAPP_DATABASE__POOL_SIZE=12\n",
    )
    .unwrap();

    with_test_config_home_env_and_current_dir(
        &home,
        &app_dir,
        &[
            ("APP_HOST", None),
            ("APP_PORT", None),
            ("APP_DATABASE__POOL_SIZE", None),
        ],
        || {
            let conf: EnvConf = read("stock", Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

            assert_eq!(conf.host, "from-dotenv");
            assert_eq!(conf.port, 9091);
            assert_eq!(conf.database.pool_size, 12);
        },
    );
}

#[test]
fn dotenv_does_not_override_existing_environment() {
    let home = temp_dir();
    let app_dir = temp_dir();
    std::fs::write(app_dir.join(".env"), "APP_HOST=from-dotenv\n").unwrap();

    with_test_config_home_env_and_current_dir(
        &home,
        &app_dir,
        &[("APP_HOST", Some("from-env"))],
        || {
            let conf: EnvConf = read("stock", Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

            assert_eq!(conf.host, "from-env");
        },
    );
}

#[test]
fn dotenv_can_be_disabled() {
    let home = temp_dir();
    let app_dir = temp_dir();
    std::fs::write(app_dir.join(".env"), "APP_HOST=from-dotenv\n").unwrap();

    with_test_config_home_env_and_current_dir(&home, &app_dir, &[("APP_HOST", None)], || {
        let options = ReadOptions::with_env_prefix("APP_").without_dotenv();
        let conf: EnvConf = read("stock", Some(options)).unwrap();

        assert_eq!(conf.host, EnvConf::default().host);
    });
}

#[test]
fn dotenv_can_be_loaded_from_explicit_path() {
    let home = temp_dir();
    let app_dir = temp_dir();
    let dotenv_path = app_dir.join(".env.local");
    std::fs::write(&dotenv_path, "APP_HOST=from-explicit-dotenv\n").unwrap();

    with_test_config_home_env_and_current_dir(&home, &app_dir, &[("APP_HOST", None)], || {
        let options = ReadOptions::with_env_prefix("APP_").with_dotenv_path(&dotenv_path);
        let conf: EnvConf = read("stock", Some(options)).unwrap();

        assert_eq!(conf.host, "from-explicit-dotenv");
    });
}
