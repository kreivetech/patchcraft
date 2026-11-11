use thiserror::Error;

#[derive(Error, Debug)]
pub enum PatchError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsafe path traversal detected: {0}")]
    UnsafePath(String),

    #[error("Search target block not found in file: {0}")]
    SearchNotFound(String),

    #[error("Failed to parse patch: {0}")]
    ParseError(String),
}
