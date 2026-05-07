# cloudiful-config

Small serde-based configuration helpers for the common app case:

- Read typed config from the platform-default user config directory.
- Create a missing `config.toml` from `T::default()`.
- Apply optional environment variable overrides on top of the file.
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

## API

The public API is intentionally small:

- `read(app_name, options)`
- `save(app_name, config)`
- `ReadOptions`

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

## Environment variable rules

- Pass `Some(ReadOptions::with_env_prefix("APP_"))` to `read(...)` when you want env overrides.
- Keys must start with the configured prefix.
- The suffix after the prefix is lowercased before matching fields.
- `__` creates nested objects.
- Values are parsed as JSON literals first.
- If JSON parsing fails, the raw value is treated as a string.
- Arrays must be provided as full JSON values such as `["a","b"]`.
- Array index syntax is not supported.
