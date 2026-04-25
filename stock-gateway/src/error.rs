use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal error: {0}")]
    Internal(String),
}
