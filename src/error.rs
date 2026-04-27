use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum OneDriveApiError {
    #[error("request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("upstream returned status {status}: {body}")]
    UpstreamStatus { status: u16, body: String },
    #[error("failed to build Graph URL: {0}")]
    GraphUrlBuild(String),
    #[error("invalid expires_in value: {0}")]
    InvalidExpiresIn(i64),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub status: u16,
    pub request_path: String,
    pub message: String,
}

impl ErrorMessage {
    pub fn new(
        status: StatusCode,
        request_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: status.as_u16(),
            request_path: request_path.into(),
            message: message.into(),
        }
    }

    pub fn with_request_path(mut self, request_path: impl Into<String>) -> Self {
        self.request_path = request_path.into();
        self
    }
}

impl From<OneDriveApiError> for ErrorMessage {
    fn from(error: OneDriveApiError) -> Self {
        let status = match &error {
            OneDriveApiError::GraphUrlBuild(_) => StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            OneDriveApiError::RequestFailed(_) | OneDriveApiError::InvalidExpiresIn(_) => {
                StatusCode::BAD_GATEWAY.as_u16()
            }
            OneDriveApiError::UpstreamStatus { status, .. } => *status,
        };

        Self {
            status,
            request_path: String::new(),
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ErrorMessage {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(self)).into_response()
    }
}
