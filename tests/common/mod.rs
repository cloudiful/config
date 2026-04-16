#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Conf {
    pub hello: i32,
    pub name: String,
    pub list: Vec<String>,
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
pub struct JsoncStringConf {
    pub url: String,
    pub note: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct NestedConf {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct EnvConf {
    pub host: String,
    pub port: u16,
    pub debug: bool,
    pub tags: Vec<String>,
    pub database: NestedConf,
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

pub fn temp_dir() -> PathBuf {
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

pub fn temp_path(file_name: &str) -> PathBuf {
    temp_dir().join(file_name)
}

pub fn with_env_changes(vars: &[(&str, Option<&str>)], test: impl FnOnce()) {
    let _guard = env_lock().lock().unwrap();
    let previous: Vec<(String, Option<OsString>)> = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var_os(key)))
        .collect();

    for (key, value) in vars {
        match value {
            Some(value) => unsafe {
                std::env::set_var(key, value);
            },
            None => unsafe {
                std::env::remove_var(key);
            },
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

pub fn with_env_vars(vars: &[(&str, &str)], test: impl FnOnce()) {
    let vars: Vec<(&str, Option<&str>)> = vars
        .iter()
        .map(|(key, value)| (*key, Some(*value)))
        .collect();
    with_env_changes(&vars, test);
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
