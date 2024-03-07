use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    // #[error("Other error: {0}")]
    // OtherError(String),
    #[error("Open file error")]
    PickFileError,
}

impl Clone for EditorError {
    fn clone(&self) -> Self {
        match self {
            EditorError::IoError(e) => {
                EditorError::IoError(io::Error::new(e.kind(), e.to_string()))
            }
            // EditorError::OtherError(e) => EditorError::OtherError(e.clone()),
            EditorError::PickFileError => EditorError::PickFileError,
        }
    }
}
