use std::fs::{self, File};
use std::io::Write;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileType {
    TOML,
    JSON,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileFormat {
    Toml,
    Json,
    Jsonc,
}

impl FileFormat {
    fn from_path(path: &Path) -> io::Result<Self> {
        match path.extension().and_then(|suffix| suffix.to_str()) {
            Some("toml") => Ok(Self::Toml),
            Some("json") => Ok(Self::Json),
            Some("jsonc") => Ok(Self::Jsonc),
            _ => Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "config file type not supported for {} (expected .toml, .json, or .jsonc)",
                    path.display()
                ),
            )),
        }
    }

    fn write_type(self) -> FileType {
        match self {
            Self::Toml => FileType::TOML,
            Self::Json | Self::Jsonc => FileType::JSON,
        }
    }
}

pub(crate) fn infer_file_type(path: &Path) -> io::Result<FileType> {
    Ok(FileFormat::from_path(path)?.write_type())
}

impl FileType {
    fn as_str(self) -> &'static str {
        match self {
            Self::TOML => "TOML",
            Self::JSON => "JSON",
        }
    }
}

fn serialize_config<T>(config: &T, file_type: FileType, path: &Path) -> io::Result<String>
where
    T: serde::Serialize + ?Sized,
{
    match file_type {
        FileType::TOML => toml::to_string_pretty(config).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "failed to serialize TOML config for {}: {e}",
                    path.display()
                ),
            )
        }),
        FileType::JSON => serde_json::to_string_pretty(config).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "failed to serialize JSON config for {}: {e}",
                    path.display()
                ),
            )
        }),
    }
}

pub(crate) fn write_config<T>(path: &Path, config: &T, file_type: FileType) -> io::Result<()>
where
    T: serde::Serialize + ?Sized,
{
    let inferred_type = infer_file_type(path)?;
    if inferred_type != file_type {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "refusing to write {} config to {} because its extension expects {}",
                file_type.as_str(),
                path.display(),
                inferred_type.as_str()
            ),
        ));
    }

    let content = serialize_config(config, file_type, path)?;
    atomic_write(path, &content)
}

fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    if path.is_dir() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("refusing to write config to directory {}", path.display()),
        ));
    }

    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to create config directory {} for {}: {err}",
                    dir.display(),
                    path.display()
                ),
            )
        })?;
    }

    let temp_path = temporary_path_for(path);
    let write_result = (|| -> io::Result<()> {
        let mut file = File::create(&temp_path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to create temporary config file {} for {}: {err}",
                    temp_path.display(),
                    path.display()
                ),
            )
        })?;
        file.write_all(content.as_bytes()).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to write temporary config file {} for {}: {err}",
                    temp_path.display(),
                    path.display()
                ),
            )
        })?;
        file.sync_all().map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to sync temporary config file {} for {}: {err}",
                    temp_path.display(),
                    path.display()
                ),
            )
        })?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }

    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(io::Error::new(
            err.kind(),
            format!(
                "failed to replace config file {} with {}: {err}",
                path.display(),
                temp_path.display()
            ),
        ));
    }

    Ok(())
}

fn temporary_path_for(path: &Path) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let temp_name = format!(".{file_name}.{}.{}.tmp", std::process::id(), unique);

    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(temp_name),
        _ => PathBuf::from(temp_name),
    }
}

fn strip_jsonc_comments(content: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Normal,
        InString,
        Escaped,
        LineComment,
        BlockComment,
    }

    let mut output = String::with_capacity(content.len());
    let mut state = State::Normal;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match state {
            State::Normal => {
                if ch == '"' {
                    output.push(ch);
                    state = State::InString;
                } else if ch == '/' && matches!(chars.peek(), Some('/')) {
                    output.push(' ');
                    output.push(' ');
                    chars.next();
                    state = State::LineComment;
                } else if ch == '/' && matches!(chars.peek(), Some('*')) {
                    output.push(' ');
                    output.push(' ');
                    chars.next();
                    state = State::BlockComment;
                } else {
                    output.push(ch);
                }
            }
            State::InString => {
                output.push(ch);
                if ch == '\\' {
                    state = State::Escaped;
                } else if ch == '"' {
                    state = State::Normal;
                }
            }
            State::Escaped => {
                output.push(ch);
                state = State::InString;
            }
            State::LineComment => {
                if ch == '\n' {
                    output.push('\n');
                    state = State::Normal;
                } else {
                    output.push(' ');
                }
            }
            State::BlockComment => {
                if ch == '*' && matches!(chars.peek(), Some('/')) {
                    output.push(' ');
                    output.push(' ');
                    chars.next();
                    state = State::Normal;
                } else if ch == '\n' {
                    output.push('\n');
                } else {
                    output.push(' ');
                }
            }
        }
    }

    output
}

pub(crate) fn read_config<T>(path: &Path) -> Result<T, io::Error>
where
    T: serde::de::DeserializeOwned,
{
    let format = FileFormat::from_path(path)?;
    let content = fs::read_to_string(path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("failed to read config {}: {err}", path.display()),
        )
    })?;

    match format {
        FileFormat::Toml => toml::from_str(&content).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("failed to parse TOML config {}: {e}", path.display()),
            )
        }),
        FileFormat::Json => serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("failed to parse JSON config {}: {e}", path.display()),
            )
        }),
        FileFormat::Jsonc => {
            let json_content = strip_jsonc_comments(&content);
            serde_json::from_str(&json_content).map_err(|e| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to parse JSONC config {}: {e}", path.display()),
                )
            })
        }
    }
}
