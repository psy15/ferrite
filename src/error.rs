use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeriteError {
    #[error("bencode error: {0}")]
    Bencode(String),

    #[error("torrent parse error: {0}")]
    TorrentParse(String),

    #[error("tracker error: {0}")]
    Tracker(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, FeriteError>;
