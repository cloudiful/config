use std::collections::BTreeMap;
use std::io::{self, ErrorKind};

pub(crate) fn resolve_secret_refs(root: &mut serde_json::Value) -> io::Result<()> {
    let mut path = Vec::new();
    resolve_value(root, &mut path)
}

fn resolve_value(value: &mut serde_json::Value, path: &mut Vec<PathSegment>) -> io::Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, nested) in map {
                path.push(PathSegment::Field(key.clone()));
                let result = resolve_value(nested, path);
                path.pop();
                result?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for (index, nested) in items.iter_mut().enumerate() {
                path.push(PathSegment::Index(index));
                let result = resolve_value(nested, path);
                path.pop();
                result?;
            }
            Ok(())
        }
        serde_json::Value::String(raw) if raw.starts_with("secret://") => {
            let resolved = resolve_secret_ref(raw, path)?;
            *value = serde_json::Value::String(resolved);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn resolve_secret_ref(raw: &str, path: &[PathSegment]) -> io::Result<String> {
    let secret_ref = SecretRef::parse(raw, path)?;

    match secret_ref.provider.as_str() {
        "keyring" => resolve_keyring_secret(&secret_ref, path),
        #[cfg(test)]
        "test" => resolve_test_secret(&secret_ref, path),
        provider => Err(path_error(
            path,
            ErrorKind::InvalidInput,
            format!("unsupported secret provider {provider}"),
        )),
    }
}

fn resolve_keyring_secret(secret_ref: &SecretRef, path: &[PathSegment]) -> io::Result<String> {
    let service = secret_ref.required_param("service", path)?;
    let user = secret_ref.required_param("user", path)?;

    #[cfg(feature = "keyring")]
    {
        use keyring_core::{Entry, Error};

        let _ = keyring::use_native_store(false);

        let entry = Entry::new(service, user).map_err(|err| {
            path_error(
                path,
                ErrorKind::InvalidInput,
                format!("failed to construct keyring entry via keyring: {err}"),
            )
        })?;

        entry.get_password().map_err(|err| {
            let detail = match err {
                Error::NoEntry => "no entry found".to_string(),
                Error::Ambiguous(_) => "multiple matching entries found".to_string(),
                Error::NoStorageAccess(_) => "storage access denied".to_string(),
                Error::BadEncoding(_) => "stored secret is not valid UTF-8".to_string(),
                Error::BadDataFormat(_, _) => "stored secret has invalid data format".to_string(),
                Error::BadStoreFormat(reason) => {
                    format!("store data is malformed: {reason}")
                }
                Error::Invalid(attribute, reason) => {
                    format!("invalid {attribute}: {reason}")
                }
                Error::TooLong(attribute, limit) => {
                    format!("{attribute} exceeds platform limit {limit}")
                }
                Error::NoDefaultStore => "no default keyring store configured".to_string(),
                Error::NotSupportedByStore(reason) => {
                    format!("operation not supported by store: {reason}")
                }
                Error::PlatformFailure(_) => "platform keyring failure".to_string(),
                #[allow(unreachable_patterns)]
                other => other.to_string(),
            };

            path_error(
                path,
                ErrorKind::NotFound,
                format!("failed to resolve secret via keyring: {detail}"),
            )
        })
    }

    #[cfg(not(feature = "keyring"))]
    {
        let _ = (service, user);
        Err(path_error(
            path,
            ErrorKind::Unsupported,
            "failed to resolve secret via keyring: keyring feature is not enabled".to_string(),
        ))
    }
}

fn path_error(path: &[PathSegment], kind: ErrorKind, message: String) -> io::Error {
    io::Error::new(
        kind,
        format!(
            "failed to resolve secret at {}: {message}",
            display_path(path)
        ),
    )
}

fn display_path(path: &[PathSegment]) -> String {
    if path.is_empty() {
        return "<root>".to_string();
    }

    let mut output = String::new();
    for segment in path {
        match segment {
            PathSegment::Field(name) => {
                if !output.is_empty() {
                    output.push('.');
                }
                output.push_str(name);
            }
            PathSegment::Index(index) => {
                output.push('[');
                output.push_str(&index.to_string());
                output.push(']');
            }
        }
    }

    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SecretRef {
    provider: String,
    params: BTreeMap<String, String>,
}

impl SecretRef {
    fn parse(raw: &str, path: &[PathSegment]) -> io::Result<Self> {
        let Some(rest) = raw.strip_prefix("secret://") else {
            return Err(path_error(
                path,
                ErrorKind::InvalidInput,
                "secret reference must start with secret://".to_string(),
            ));
        };

        let (provider, query) = rest.split_once('?').ok_or_else(|| {
            path_error(
                path,
                ErrorKind::InvalidInput,
                "secret reference must include query parameters".to_string(),
            )
        })?;

        if provider.is_empty() {
            return Err(path_error(
                path,
                ErrorKind::InvalidInput,
                "secret provider must not be empty".to_string(),
            ));
        }

        let params = parse_query(query, path)?;
        Ok(Self {
            provider: provider.to_string(),
            params,
        })
    }

    fn required_param<'a>(&'a self, key: &str, path: &[PathSegment]) -> io::Result<&'a str> {
        self.params
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                path_error(
                    path,
                    ErrorKind::InvalidInput,
                    format!("missing required parameter {key}"),
                )
            })
    }
}

fn parse_query(query: &str, path: &[PathSegment]) -> io::Result<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let (key, value) = pair.split_once('=').ok_or_else(|| {
            path_error(
                path,
                ErrorKind::InvalidInput,
                "secret query parameter must be key=value".to_string(),
            )
        })?;

        if key.is_empty() {
            return Err(path_error(
                path,
                ErrorKind::InvalidInput,
                "secret query parameter key must not be empty".to_string(),
            ));
        }

        params.insert(percent_decode(key, path)?, percent_decode(value, path)?);
    }

    Ok(params)
}

fn percent_decode(raw: &str, path: &[PathSegment]) -> io::Result<String> {
    let mut output = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                if index + 2 >= bytes.len() {
                    return Err(path_error(
                        path,
                        ErrorKind::InvalidInput,
                        "invalid percent-encoding in secret reference".to_string(),
                    ));
                }

                let decoded = decode_hex_pair(bytes[index + 1], bytes[index + 2]).ok_or_else(|| {
                    path_error(
                        path,
                        ErrorKind::InvalidInput,
                        "invalid percent-encoding in secret reference".to_string(),
                    )
                })?;
                output.push(decoded);
                index += 3;
            }
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8(output).map_err(|_| {
        path_error(
            path,
            ErrorKind::InvalidInput,
            "secret reference contains invalid UTF-8".to_string(),
        )
    })
}

fn decode_hex_pair(high: u8, low: u8) -> Option<u8> {
    let high = decode_hex_nibble(high)?;
    let low = decode_hex_nibble(low)?;
    Some((high << 4) | low)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PathSegment {
    Field(String),
    Index(usize),
}

#[cfg(test)]
fn resolve_test_secret(secret_ref: &SecretRef, path: &[PathSegment]) -> io::Result<String> {
    secret_ref.required_param("value", path).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::resolve_secret_refs;

    #[test]
    fn replaces_secret_values_in_nested_objects_and_arrays() {
        let mut value = serde_json::json!({
            "database": {
                "password": "secret://test?value=db-pass"
            },
            "tokens": [
                "secret://test?value=api-token"
            ]
        });

        resolve_secret_refs(&mut value).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "database": {
                    "password": "db-pass"
                },
                "tokens": ["api-token"]
            })
        );
    }

    #[test]
    fn rejects_missing_query_parameters() {
        let mut value = serde_json::json!({
            "database": {
                "password": "secret://keyring?service=stock"
            }
        });

        let err = resolve_secret_refs(&mut value).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("missing required parameter user"));
    }

    #[test]
    fn rejects_invalid_secret_uri() {
        let mut value = serde_json::json!({
            "password": "secret://keyring"
        });

        let err = resolve_secret_refs(&mut value).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("query parameters"));
    }
}
