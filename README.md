# cloudiful-config

Small serde-based configuration helpers for local TOML, JSON, and JSONC files.

`cloudiful-config` focuses on the small but common path:

- Read typed config from `.toml`, `.json`, or `.jsonc`.
- Create a missing config file from `T::default()`.
- Apply environment variable overrides on top of defaults or file-backed config.
- Save config back atomically as TOML or JSON.

It is intentionally narrower than crates like `config-rs` or `Figment`. This
crate is for applications that already know which file to read and want a small
read/write helper around Serde types.

## Install

```toml
[dependencies]
cloudiful-config = "0.4.1"
```

## Read modes

`read(ConfigSource)` stays the main entrypoint:

- `ConfigSource::File(path)`: read from disk, creating the file from `T::default()` if it does not exist yet.
- `ConfigSource::Env { prefix }`: start from `T::default()` and apply matching environment variables.
- `ConfigSource::FileWithEnv { path, prefix }`: read from disk first, then apply environment variables on top.

Use `read_existing(path)` when the file must already exist.

Use `read_or_create_default(path)` when you want the "create on first run" behavior explicitly without environment overrides.

## Automatic config paths

If you do not want to hardcode a full path, use the platform-aware helpers:

- `config_dir("my-app")`
- `config_path("my-app", "config.toml")`

They resolve to the standard per-user config location for the current OS:

- Linux and other XDG platforms: `$XDG_CONFIG_HOME/my-app` or `~/.config/my-app`
- macOS: `~/Library/Application Support/my-app`
- Windows: `%APPDATA%\\my-app`

Example:

```rust,no_run
use cloudiful_config::{config_path, read_or_create_default};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
struct AppConfig {
    port: u16,
}

let path = config_path("my-app", "config.toml").unwrap();
let config: AppConfig = read_or_create_default(&path).unwrap();
```

## Writing config

`save_inferred(path, config)` is the recommended write API. It infers the output format from the path extension:

- `.toml` writes TOML
- `.json` writes JSON
- `.jsonc` also writes standard JSON

`save(path, config, file_type)` is still available for explicit format control, but the format must match the path extension. For example, JSON cannot be written to a `.toml` path.

All writes are atomic: the crate writes to a temporary file in the same directory and then renames it into place.

## Environment variable rules

- Keys must start with the configured prefix.
- The suffix after the prefix is lowercased before matching fields.
- `__` creates nested objects.
- Values are parsed as JSON literals first.
- If JSON parsing fails, the raw value is treated as a string.
- Arrays must be provided as full JSON values such as `["a","b"]`.
- Array index syntax is not supported.

## JSONC behavior

`.jsonc` inputs support `//` and `/* ... */` comments while reading.

`.jsonc` outputs are written as standard JSON. Comments are not preserved.
