use cloudiful_config::{read, save, sqlite_store};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
struct AppConfig {
    host: String,
    port: u16,
}

fn setup_db() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn sqlite_api_round_trips_configuration() {
    let conn = setup_db();
    let mut store = sqlite_store(&conn, "stock");

    save(
        &mut store,
        &AppConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
        },
    )
    .unwrap();

    let mut store = sqlite_store(&conn, "stock");
    let conf: AppConfig = read(&mut store, None).unwrap();
    assert_eq!(
        conf,
        AppConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    );
}
