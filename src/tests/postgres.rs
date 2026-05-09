use crate::{postgres_store, postgres_store_with_table, read, save};
use postgres::NoTls;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
struct PgConf {
    host: String,
    port: u16,
}

#[test]
fn postgres_store_type_is_usable_with_unified_api() {
    fn _accepts_store(mut client: postgres::Client) {
        let mut store = postgres_store(&mut client, "stock");

        let _ = save(
            &mut store,
            &PgConf {
                host: "localhost".to_string(),
                port: 5432,
            },
        );
        let _: Result<PgConf, _> = read(&mut store, None);
    }

    let _ = _accepts_store;
}

#[test]
fn postgres_round_trip_when_test_url_is_available() {
    let Some(url) = std::env::var_os("TEST_POSTGRES_URL") else {
        return;
    };

    let mut client = postgres::Client::connect(&url.to_string_lossy(), NoTls).unwrap();
    client
        .batch_execute(
            "DROP TABLE IF EXISTS app_configs;
             DROP TABLE IF EXISTS bad_configs;",
        )
        .unwrap();

    let mut store = postgres_store(&mut client, "stock");

    save(
        &mut store,
        &PgConf {
            host: "127.0.0.1".to_string(),
            port: 5432,
        },
    )
    .unwrap();

    let conf: PgConf = read(&mut store, None).unwrap();
    assert_eq!(
        conf,
        PgConf {
            host: "127.0.0.1".to_string(),
            port: 5432,
        }
    );
}

#[test]
fn postgres_conflict_detection_rejects_wrong_table_shape_when_test_url_is_available() {
    let Some(url) = std::env::var_os("TEST_POSTGRES_URL") else {
        return;
    };

    let mut client = postgres::Client::connect(&url.to_string_lossy(), NoTls).unwrap();
    client
        .batch_execute(
            "DROP TABLE IF EXISTS bad_configs;
             CREATE TABLE bad_configs (
                id BIGSERIAL PRIMARY KEY,
                payload TEXT NOT NULL
             );",
        )
        .unwrap();

    let mut store = postgres_store_with_table(&mut client, "stock", "bad_configs");
    let err = read::<PgConf>(&mut store, None).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(err.to_string().contains("expected config schema"));
}
