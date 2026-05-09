use std::io;

use postgres::Client;
use rusqlite::{params, Connection, OptionalExtension};

use crate::format;
use crate::ConfigSource;

pub const DEFAULT_CONFIG_TABLE: &str = "app_configs";

pub struct SqliteConfigStore<'conn> {
    conn: &'conn Connection,
    table_name: String,
    app_name: String,
}

impl<'conn> SqliteConfigStore<'conn> {
    pub fn new(conn: &'conn Connection, app_name: &str, table_name: Option<&str>) -> Self {
        Self {
            conn,
            table_name: table_name.unwrap_or(DEFAULT_CONFIG_TABLE).to_string(),
            app_name: app_name.to_string(),
        }
    }

    fn ensure_schema(&self) -> io::Result<()> {
        self.validate_identifier(&self.table_name)?;
        self.conn
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        app_name TEXT PRIMARY KEY,
                        config_json TEXT NOT NULL,
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    )",
                    self.table_name
                ),
                [],
            )
            .map_err(to_io_error_sqlite)?;
        self.detect_conflict()?;
        Ok(())
    }

    fn detect_conflict(&self) -> io::Result<()> {
        let pragma = format!("PRAGMA table_info({})", self.table_name);
        let mut stmt = self.conn.prepare(&pragma).map_err(to_io_error_sqlite)?;
        let mut rows = stmt.query([]).map_err(to_io_error_sqlite)?;

        let mut has_app_name = false;
        let mut has_config_json = false;

        while let Some(row) = rows.next().map_err(to_io_error_sqlite)? {
            let name: String = row.get(1).map_err(to_io_error_sqlite)?;
            let pk: i64 = row.get(5).map_err(to_io_error_sqlite)?;
            if name == "app_name" && pk == 1 {
                has_app_name = true;
            }
            if name == "config_json" {
                has_config_json = true;
            }
        }

        if has_app_name && has_config_json {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "refusing to use sqlite table {} because it does not match the expected config schema",
                    self.table_name
                ),
            ))
        }
    }

    fn validate_identifier(&self, ident: &str) -> io::Result<()> {
        if ident.is_empty()
            || !ident
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid sqlite config table name {ident}"),
            ));
        }
        Ok(())
    }
}

impl ConfigSource for SqliteConfigStore<'_> {
    fn source_name(&self) -> String {
        format!("sqlite:{}:{}", self.table_name, self.app_name)
    }

    fn read_value(&mut self) -> io::Result<Option<serde_json::Value>> {
        self.ensure_schema()?;
        let query = format!(
            "SELECT config_json FROM {} WHERE app_name = ?1",
            self.table_name
        );
        let mut stmt = self.conn.prepare(&query).map_err(to_io_error_sqlite)?;
        let value: Option<String> = stmt
            .query_row([&self.app_name], |row| row.get(0))
            .optional()
            .map_err(to_io_error_sqlite)?;

        match value {
            Some(raw) => {
                format::parse_config_value(&raw, format::ConfigFormat::Json, &self.source_name())
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn write_config<T>(&mut self, config: &T) -> io::Result<()>
    where
        T: serde::Serialize,
    {
        self.ensure_schema()?;
        let raw = format::serialize_config(config, format::ConfigFormat::Json, &self.source_name())?;
        let query = format!(
            "INSERT INTO {} (app_name, config_json) VALUES (?1, ?2)
             ON CONFLICT(app_name) DO UPDATE
             SET config_json = excluded.config_json, updated_at = CURRENT_TIMESTAMP",
            self.table_name
        );
        self.conn
            .execute(&query, params![self.app_name, raw])
            .map_err(to_io_error_sqlite)?;
        Ok(())
    }
}

pub struct PostgresConfigStore<'client> {
    client: &'client mut Client,
    table_name: String,
    app_name: String,
}

impl<'client> PostgresConfigStore<'client> {
    pub fn new(client: &'client mut Client, app_name: &str, table_name: Option<&str>) -> Self {
        Self {
            client,
            table_name: table_name.unwrap_or(DEFAULT_CONFIG_TABLE).to_string(),
            app_name: app_name.to_string(),
        }
    }

    fn ensure_schema(&mut self) -> io::Result<()> {
        self.validate_identifier(&self.table_name)?;
        self.client
            .batch_execute(&format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    app_name TEXT PRIMARY KEY,
                    config_json TEXT NOT NULL,
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                )",
                self.table_name
            ))
            .map_err(to_io_error_postgres)?;
        self.detect_conflict()?;
        Ok(())
    }

    fn detect_conflict(&mut self) -> io::Result<()> {
        let row = self
            .client
            .query_one(
                "SELECT
                    EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_name = $1 AND column_name = 'app_name'
                    ),
                    EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_name = $1 AND column_name = 'config_json'
                    )",
                &[&self.table_name],
            )
            .map_err(to_io_error_postgres)?;

        let has_app_name: bool = row.get(0);
        let has_config_json: bool = row.get(1);

        if has_app_name && has_config_json {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "refusing to use postgres table {} because it does not match the expected config schema",
                    self.table_name
                ),
            ))
        }
    }

    fn validate_identifier(&self, ident: &str) -> io::Result<()> {
        if ident.is_empty()
            || !ident
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid postgres config table name {ident}"),
            ));
        }
        Ok(())
    }
}

impl ConfigSource for PostgresConfigStore<'_> {
    fn source_name(&self) -> String {
        format!("postgres:{}:{}", self.table_name, self.app_name)
    }

    fn read_value(&mut self) -> io::Result<Option<serde_json::Value>> {
        self.ensure_schema()?;
        let query = format!(
            "SELECT config_json FROM {} WHERE app_name = $1",
            self.table_name
        );
        let row = self
            .client
            .query_opt(&query, &[&self.app_name])
            .map_err(to_io_error_postgres)?;

        match row {
            Some(row) => {
                let raw: String = row.get(0);
                format::parse_config_value(&raw, format::ConfigFormat::Json, &self.source_name())
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn write_config<T>(&mut self, config: &T) -> io::Result<()>
    where
        T: serde::Serialize,
    {
        self.ensure_schema()?;
        let raw = format::serialize_config(config, format::ConfigFormat::Json, &self.source_name())?;
        let query = format!(
            "INSERT INTO {} (app_name, config_json) VALUES ($1, $2)
             ON CONFLICT (app_name) DO UPDATE
             SET config_json = EXCLUDED.config_json, updated_at = NOW()",
            self.table_name
        );
        self.client
            .execute(&query, &[&self.app_name, &raw])
            .map_err(to_io_error_postgres)?;
        Ok(())
    }
}

fn to_io_error_sqlite(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}

fn to_io_error_postgres(err: postgres::Error) -> io::Error {
    io::Error::other(err)
}

pub fn sqlite_store<'conn>(conn: &'conn Connection, app_name: &str) -> SqliteConfigStore<'conn> {
    SqliteConfigStore::new(conn, app_name, None)
}

pub fn sqlite_store_with_table<'conn>(
    conn: &'conn Connection,
    app_name: &str,
    table_name: &str,
) -> SqliteConfigStore<'conn> {
    SqliteConfigStore::new(conn, app_name, Some(table_name))
}

pub fn postgres_store<'client>(
    client: &'client mut Client,
    app_name: &str,
) -> PostgresConfigStore<'client> {
    PostgresConfigStore::new(client, app_name, None)
}

pub fn postgres_store_with_table<'client>(
    client: &'client mut Client,
    app_name: &str,
    table_name: &str,
) -> PostgresConfigStore<'client> {
    PostgresConfigStore::new(client, app_name, Some(table_name))
}
