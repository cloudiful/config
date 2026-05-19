# cloudiful-config

Small serde-based configuration helpers for the common app case:

- Read typed config from the platform-default user config directory.
- Create a missing `config.toml` from `T::default()`.
- Read and write whole config blobs from Postgres through the same `read`/`save` API.
- Use a default config table named `app_configs` unless you override it.
- Apply optional environment variable overrides.
- Resolve explicit `secret://...` references before deserializing.
- Save config back atomically as TOML.

For an app named `stock`, the default file path is:

- Linux and other XDG platforms: `$XDG_CONFIG_HOME/stock/config.toml` or `~/.config/stock/config.toml`
- macOS: `~/Library/Application Support/stock/config.toml`
- Windows: `%APPDATA%\\stock\\config.toml`

## Install

```toml
[dependencies]
cloudiful-config = "0.6.1"
```

Enable `keyring` when you want `secret://keyring?...` references to resolve through the system credential store:

```toml
[dependencies]
cloudiful-config = { version = "0.6.1", features = ["keyring"] }
```

## Usage

Example:

```rust,no_run
use cloudiful_config::{ReadOptions, read, save};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
struct AppConfig {
    port: u16,
    debug: bool,
}

let config: AppConfig = read("stock", None).unwrap();
let config: AppConfig = read("stock", Some(ReadOptions::with_env_prefix("STOCK_"))).unwrap();
save("stock", config).unwrap();
```

Postgres example:

```rust,no_run
use cloudiful_config::{postgres_store, read, save};
use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
struct AppConfig {
    host: String,
    port: u16,
}

let mut client = Client::connect("host=localhost user=postgres dbname=app", NoTls).unwrap();
let mut store = postgres_store(&mut client, "stock");

let config: AppConfig = read(&mut store, None).unwrap();
save(&mut store, config).unwrap();
```

Default table schema:

```sql
CREATE TABLE app_configs (
  app_name TEXT PRIMARY KEY,
  config_json TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
```

## Read behavior

- `read("app-name", ...)` loads the default config file, creates it from `T::default()` if missing, applies optional env overrides, resolves secret references, and then deserializes into `T`.
- `read(store, ...)` does the same flow against the provided store.
- `save("app-name", config)` writes TOML back to the default path.
- `save(store, config)` writes the full config blob to the provided store.
- `postgres_store(client, "stock")` uses the default `app_configs` table.
- `postgres_store_with_table(...)` lets you override the table name.
- When the target table already exists but does not match the expected config schema, the store returns an error instead of writing into it.

### Environment overrides

- Pass `Some(ReadOptions::with_env_prefix("APP_"))` to enable env overrides.
- `read(...)` loads `.env` from the current directory by default before applying env overrides.
- `.env` is optional and never overrides existing environment variables.
- Use `ReadOptions::default().without_dotenv()` or `ReadOptions::with_env_prefix("APP_").without_dotenv()` to opt out.
- Use `with_dotenv_path(path)` to load a specific dotenv file.
- Keys must start with the configured prefix.
- The suffix after the prefix is lowercased before matching fields.
- `__` creates nested objects.
- Values are parsed as JSON literals first.
- If JSON parsing fails, the raw value is treated as a string.
- Arrays must be provided as full JSON values such as `["a","b"]`.
- Array index syntax is not supported.

### Secret references

- Only explicit string values starting with `secret://` are resolved.
- Secret resolution is strict: invalid or missing secrets return an error.
- `save(...)` never resolves or writes secrets to the system store. If a config value is `secret://...`, it is saved as that literal string.
- Query parameters support percent-encoding.

Supported provider:

- `keyring` via the optional `keyring` feature

Reference format:

```text
secret://keyring?service=<service>&user=<user>
```

- `service` is required.
- `user` is required.

Example TOML:

```toml
[database]
user = "app"
password = "secret://keyring?service=stock&user=db-prod"
```

Example environment override:

```bash
APP_DATABASE__PASSWORD=secret://keyring?service=stock&user=db-prod
```

On macOS this resolves through Keychain via the Rust `keyring` crate. The same reference format also works for other platforms supported by `keyring`.
