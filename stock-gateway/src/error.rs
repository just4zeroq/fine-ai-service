use salvo::http::StatusCode;
use salvo::Response;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::RateLimited(_) => "RATE_LIMITED",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Database(_) => "INTERNAL_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

pub fn write_error_response(res: &mut Response, error: &AppError) {
    let body = serde_json::json!({
        "error": {
            "code": error.error_code(),
            "message": error.to_string()
        }
    });
    res.status_code(error.status_code());
    res.render(serde_json::to_string(&body).unwrap());
}
