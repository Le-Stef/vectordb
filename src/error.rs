use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorDbError {
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Collection already exists: {0}")]
    CollectionAlreadyExists(String),

    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Vector not found: {0}")]
    VectorNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, VectorDbError>;

impl From<serde_json::Error> for VectorDbError {
    fn from(err: serde_json::Error) -> Self {
        VectorDbError::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for VectorDbError {
    fn from(err: bincode::Error) -> Self {
        VectorDbError::Serialization(err.to_string())
    }
}
