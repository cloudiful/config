use std::io;

use postgres::Client;

use crate::format;
use crate::ConfigSource;

pub const DEFAULT_CONFIG_TABLE: &str = "app_configs";

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

fn to_io_error_postgres(err: postgres::Error) -> io::Error {
    io::Error::other(err)
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
