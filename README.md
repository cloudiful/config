# cloudiful-config

Small serde-based configuration helpers for the common app case:

- Read typed config from the platform-default user config directory.
- Create a missing `config.toml` from `T::default()`.
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
cloudiful-config = "0.5.0"
```

Enable `keyring` when you want `secret://keyring?...` references to resolve through the system credential store:

```toml
[dependencies]
cloudiful-config = { version = "0.5.0", features = ["keyring"] }
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

## Read behavior

- `read(...)` loads the default config file, creates it from `T::default()` if missing, applies optional env overrides, resolves secret references, and then deserializes into `T`.
- `save(...)` writes TOML back to the same default path.

### Environment overrides

- Pass `Some(ReadOptions::with_env_prefix("APP_"))` to enable env overrides.
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
