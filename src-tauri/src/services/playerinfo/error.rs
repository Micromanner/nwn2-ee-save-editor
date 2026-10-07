use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerInfoParseError {
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    #[error("Invalid string at position {position}: {reason}")]
    InvalidString { position: u64, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PlayerInfoResult<T> = Result<T, PlayerInfoParseError>;
