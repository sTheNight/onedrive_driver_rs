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
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error("upstream returned status {status}: {body}")]
    UpstreamStatus { status: u16, body: String },
    #[error("failed to build Graph URL: {0}")]
    GraphUrlBuild(String),
    #[error("invalid expires_in value: {0}")]
    InvalidExpiresIn(i64),
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("onedrive config is missing")]
    MissingOneDriveConfig,
    #[error("{0} is empty in onedrive config")]
    MissingOneDriveConfigField(&'static str),
    #[error("database operation failed: {0}")]
    Database(#[from] sea_orm::DbErr),
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
            OneDriveApiError::Service(ServiceError::MissingOneDriveConfig)
            | OneDriveApiError::Service(ServiceError::MissingOneDriveConfigField(_)) => {
                StatusCode::CONFLICT.as_u16()
            }
            OneDriveApiError::Service(ServiceError::Database(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR.as_u16()
            }
            OneDriveApiError::UpstreamStatus { status, .. } => *status,
        };
        let message = onedrive_api_error_message(error);

        Self {
            status,
            request_path: String::new(),
            message,
        }
    }
}

fn onedrive_api_error_message(error: OneDriveApiError) -> String {
    match error {
        OneDriveApiError::RequestFailed(err) if err.is_timeout() => {
            "OneDrive request timed out".to_string()
        }
        OneDriveApiError::RequestFailed(err) if err.is_connect() => {
            "Failed to connect to OneDrive".to_string()
        }
        OneDriveApiError::RequestFailed(_) => "OneDrive request failed".to_string(),
        OneDriveApiError::Service(ServiceError::MissingOneDriveConfig) => {
            "OneDrive config is missing".to_string()
        }
        OneDriveApiError::Service(ServiceError::MissingOneDriveConfigField(field)) => {
            format!("{field} is empty in OneDrive config")
        }
        OneDriveApiError::Service(ServiceError::Database(_)) => {
            "Database operation failed".to_string()
        }
        OneDriveApiError::UpstreamStatus { status, body } => parse_upstream_message(status, &body),
        OneDriveApiError::GraphUrlBuild(_) => {
            "Failed to build Microsoft Graph request URL".to_string()
        }
        OneDriveApiError::InvalidExpiresIn(expires_in) => {
            format!("Invalid expires_in value returned by OneDrive: {expires_in}")
        }
    }
}

fn parse_upstream_message(status: u16, body: &str) -> String {
    serde_json::from_str::<GraphErrorResponse>(body)
        .ok()
        .map(|response| response.error.into_message())
        .or_else(|| {
            serde_json::from_str::<OAuthErrorResponse>(body)
                .ok()
                .map(OAuthErrorResponse::into_message)
        })
        .unwrap_or_else(|| {
            let body = body.trim();
            if body.is_empty() {
                format!("OneDrive upstream returned HTTP {status}")
            } else {
                body.to_string()
            }
        })
}

#[derive(Debug, Deserialize)]
struct GraphErrorResponse {
    error: GraphError,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    code: Option<String>,
    message: Option<String>,
}

impl GraphError {
    fn into_message(self) -> String {
        match (self.code, self.message) {
            (Some(code), Some(message))
                if !code.trim().is_empty() && !message.trim().is_empty() =>
            {
                format!("{code}: {message}")
            }
            (_, Some(message)) if !message.trim().is_empty() => message,
            (Some(code), _) if !code.trim().is_empty() => code,
            _ => "OneDrive upstream returned an error".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
}

impl OAuthErrorResponse {
    fn into_message(self) -> String {
        match (self.error, self.error_description) {
            (Some(code), Some(description))
                if !code.trim().is_empty() && !description.trim().is_empty() =>
            {
                format!("{code}: {description}")
            }
            (_, Some(description)) if !description.trim().is_empty() => description,
            (Some(code), _) if !code.trim().is_empty() => code,
            _ => "OneDrive upstream returned an error".to_string(),
        }
    }
}

impl IntoResponse for ErrorMessage {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(self)).into_response()
    }
}
