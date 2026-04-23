use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "code", content = "details")]
pub enum CommandError {
    // Session errors
    #[error("No character loaded")]
    NoCharacterLoaded,

    #[error("No game data loaded")]
    NoGameDataLoaded,

    #[error("Character not found: {path}")]
    CharacterNotFound { path: String },

    // Validation errors
    #[error("Validation failed")]
    ValidationError { field: String, reason: String },

    #[error("Invalid value: {field}")]
    InvalidValue {
        field: String,
        expected: String,
        actual: String,
    },

    // I/O errors
    #[error("File error: {message}")]
    FileError {
        message: String,
        path: Option<String>,
        diagnostics_path: Option<String>,
    },

    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        context: Option<String>,
        diagnostics_path: Option<String>,
    },

    // Game logic errors
    #[error("Insufficient resources")]
    InsufficientResources {
        resource: String,
        required: i32,
        available: i32,
    },

    #[error("Prerequisites not met")]
    PrerequisitesNotMet { missing: Vec<String> },

    #[error("Not found: {item}")]
    NotFound { item: String },

    #[error("Already exists: {item}")]
    AlreadyExists { item: String },

    #[error("Operation failed: {operation}")]
    OperationFailed { operation: String, reason: String },

    // Generic fallback
    #[error("{0}")]
    Internal(String),
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::FileError {
            message: e.to_string(),
            path: None,
            diagnostics_path: None,
        }
    }
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        Self::Internal(s.to_string())
    }
}

impl From<crate::character::CharacterError> for CommandError {
    fn from(e: crate::character::CharacterError) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<crate::services::toolset_bridge::BridgeError> for CommandError {
    fn from(e: crate::services::toolset_bridge::BridgeError) -> Self {
        use crate::services::toolset_bridge::BridgeError;
        match e {
            BridgeError::BridgeNotFound(path) => Self::NotFound {
                item: format!("Toolset bridge: {}", path.display()),
            },
            BridgeError::InstallNotFound(path) => Self::NotFound {
                item: format!("NWN2 install: {}", path.display()),
            },
            BridgeError::InputNotFound(path) => Self::NotFound {
                item: format!("Module/campaign: {}", path.display()),
            },
            BridgeError::PlatformUnsupported(msg) => Self::OperationFailed {
                operation: "toolset bridge".to_string(),
                reason: msg,
            },
            BridgeError::NonZeroExit { code, stderr } => Self::OperationFailed {
                operation: format!("toolset bridge (exit {code})"),
                reason: stderr,
            },
            BridgeError::KilledBySignal { stderr } => Self::OperationFailed {
                operation: "toolset bridge (killed)".to_string(),
                reason: stderr,
            },
            BridgeError::ParseFailed(err) => Self::ParseError {
                message: err.to_string(),
                context: Some("toolset bridge json".to_string()),
                diagnostics_path: None,
            },
            BridgeError::Io(err) => Self::FileError {
                message: err.to_string(),
                path: None,
                diagnostics_path: None,
            },
        }
    }
}

impl From<crate::services::savegame_handler::SaveGameError> for CommandError {
    fn from(e: crate::services::savegame_handler::SaveGameError) -> Self {
        use crate::services::savegame_handler::SaveGameError;
        match e {
            SaveGameError::NotFound { path } => Self::NotFound {
                item: format!("Save game: {}", path.display()),
            },
            SaveGameError::FileNotInSave { filename } => Self::NotFound {
                item: format!("File in save: {filename}"),
            },
            SaveGameError::PermissionDenied(msg)
            | SaveGameError::BackupFailed(msg)
            | SaveGameError::RestoreFailed(msg) => Self::FileError {
                message: msg,
                path: None,
                diagnostics_path: None,
            },
            SaveGameError::Io(io_err) => Self::FileError {
                message: io_err.to_string(),
                path: None,
                diagnostics_path: None,
            },
            SaveGameError::CorruptedZip(msg)
            | SaveGameError::GffParse(msg)
            | SaveGameError::InvalidStructure(msg) => Self::ParseError {
                message: msg,
                context: None,
                diagnostics_path: None,
            },
            SaveGameError::ValidationFailed { filename, reason } => Self::ValidationError {
                field: filename,
                reason,
            },
            _ => Self::Internal(e.to_string()),
        }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_serialization() {
        let err = CommandError::NoCharacterLoaded;
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("NoCharacterLoaded"));
    }

    #[test]
    fn test_validation_error_serialization() {
        let err = CommandError::ValidationError {
            field: "deity".to_string(),
            reason: "Invalid deity name".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("ValidationError"));
        assert!(json.contains("deity"));
    }

    #[test]
    fn test_error_from_string() {
        let err: CommandError = "test error".into();
        assert!(matches!(err, CommandError::Internal(_)));
    }
}
