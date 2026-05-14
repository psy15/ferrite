use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeriteError {
    #[error("bencode error: {0}")]
    Bencode(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, FeriteError>;
