use crate::{read, save, sqlite_store, sqlite_store_with_table, ReadOptions};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
struct SqlConf {
    host: String,
    port: u16,
    database: SqlDatabase,
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
struct SqlDatabase {
    url: String,
    password: String,
}

fn setup_db() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn missing_sql_config_is_written_from_default() {
    let conn = setup_db();
    let mut store = sqlite_store(&conn, "stock");

    let conf: SqlConf = read(&mut store, None).unwrap();

    assert_eq!(conf, SqlConf::default());

    let raw: String = conn
        .query_row("SELECT config_json FROM app_configs WHERE app_name = 'stock'", [], |row| row.get(0))
        .unwrap();
    assert!(raw.contains("\"host\""));
}

#[test]
fn save_to_sqlite_overwrites_entire_blob() {
    let conn = setup_db();
    let mut store = sqlite_store(&conn, "stock");

    save(
        &mut store,
        &SqlConf {
            host: "from-file".to_string(),
            port: 8080,
            database: SqlDatabase {
                url: "postgres://db/file".to_string(),
                password: "plain".to_string(),
            },
        },
    )
    .unwrap();

    let mut store = sqlite_store(&conn, "stock");
    save(
        &mut store,
        &SqlConf {
            host: "from-db".to_string(),
            port: 9090,
            database: SqlDatabase {
                url: "postgres://db/new".to_string(),
                password: "plain-2".to_string(),
            },
        },
    )
    .unwrap();

    let mut store = sqlite_store(&conn, "stock");
    let conf: SqlConf = read(&mut store, None).unwrap();
    assert_eq!(conf.host, "from-db");
    assert_eq!(conf.port, 9090);
}

#[test]
fn sqlite_env_and_secret_processing_runs_after_db_load() {
    let conn = setup_db();
    let mut store = sqlite_store(&conn, "stock");

    save(
        &mut store,
        &SqlConf {
            host: "from-db".to_string(),
            port: 8080,
            database: SqlDatabase {
                url: "postgres://db/file".to_string(),
                password: "secret://test?value=db-pass".to_string(),
            },
        },
    )
    .unwrap();

    super::support::with_env_changes(
        &[
            ("APP_PORT", Some("9090")),
            ("APP_DATABASE__URL", Some("\"postgres://db/env\"")),
        ],
        || {
            let mut store = sqlite_store(&conn, "stock");
            let conf: SqlConf =
                read(&mut store, Some(ReadOptions::with_env_prefix("APP_"))).unwrap();

            assert_eq!(conf.port, 9090);
            assert_eq!(conf.database.url, "postgres://db/env");
            assert_eq!(conf.database.password, "db-pass");
        },
    );
}

#[test]
fn sqlite_conflict_detection_rejects_wrong_table_shape() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE bad_configs (id INTEGER PRIMARY KEY, payload TEXT NOT NULL)", [])
        .unwrap();

    let mut store = sqlite_store_with_table(&conn, "stock", "bad_configs");
    let err = read::<SqlConf>(&mut store, None).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(err.to_string().contains("expected config schema"));
}
