use std::io::{self, ErrorKind};

fn parse_env_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

fn insert_env_value(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    let mut segments = key
        .split("__")
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .peekable();

    if segments.peek().is_none() {
        return;
    }

    let mut current = root;
    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            current.insert(segment, value);
            return;
        }

        let entry = current
            .entry(segment)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

        if !entry.is_object() {
            *entry = serde_json::Value::Object(serde_json::Map::new());
        }

        current = entry.as_object_mut().expect("entry is forced to be object");
    }
}

fn env_overrides(prefix: &str) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    for (key, value) in std::env::vars() {
        let Some(stripped_key) = key.strip_prefix(prefix) else {
            continue;
        };

        insert_env_value(&mut root, stripped_key, parse_env_value(&value));
    }

    serde_json::Value::Object(root)
}

fn merge_json(base: &mut serde_json::Value, overrides: serde_json::Value) {
    match (base, overrides) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(override_map)) => {
            for (key, override_value) in override_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => merge_json(base_value, override_value),
                    None => {
                        base_map.insert(key, override_value);
                    }
                }
            }
        }
        (base_value, override_value) => {
            *base_value = override_value;
        }
    }
}

pub(crate) fn apply_env_overrides<T>(config: T, prefix: &str) -> Result<T, io::Error>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut base = serde_json::to_value(config).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "failed to serialize config before applying environment overrides for prefix {prefix}: {e}"
            ),
        )
    })?;
    let overrides = env_overrides(prefix);

    merge_json(&mut base, overrides);

    serde_json::from_value(base).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "failed to deserialize config after applying environment overrides for prefix {prefix}: {e}"
            ),
        )
    })
}
