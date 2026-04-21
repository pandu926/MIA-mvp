use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Feature disabled: {0}")]
    FeatureDisabled(String),

    #[error("Payment required: {0}")]
    PaymentRequired(String),

    #[error("Not ready: {0}")]
    NotReady(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            AppError::Redis(e) => {
                tracing::error!("Redis error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error".to_string())
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::FeatureDisabled(msg) => {
                tracing::warn!("Feature disabled: {}", msg);
                (StatusCode::SERVICE_UNAVAILABLE, msg.clone())
            }
            AppError::PaymentRequired(msg) => {
                tracing::info!("Payment required: {}", msg);
                (StatusCode::PAYMENT_REQUIRED, msg.clone())
            }
            AppError::NotReady(msg) => {
                tracing::warn!("Feature not ready: {}", msg);
                (StatusCode::NOT_IMPLEMENTED, msg.clone())
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
