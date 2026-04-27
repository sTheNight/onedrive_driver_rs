use std::sync::Arc;
use tokio::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("{0} is empty")]
    MissingEnvVar(&'static str),
    #[error("{0} is null")]
    EmptyEnvVar(&'static str),
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_at: Instant,
}

impl AccessToken {
    pub fn new(access_token: String, expires_in: u64) -> Self {
        let safe_expires_in = expires_in.saturating_sub(60);

        Self {
            access_token,
            expires_at: Instant::now() + Duration::from_secs(safe_expires_in),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub root_path: String,
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Arc<Mutex<Option<AccessToken>>>,
}

impl AppState {
    pub fn from_env() -> Result<Self, AppStateError> {
        let root_path = std::env::var("ONEDRIVE_ROOT_PATH").unwrap_or_default();
        let client_id = std::env::var("ONEDRIVE_CLIENT_ID")
            .map_err(|_| AppStateError::MissingEnvVar("ONEDRIVE_CLIENT_ID"))?;
        let client_secret = std::env::var("ONEDRIVE_CLIENT_SECRET")
            .map_err(|_| AppStateError::MissingEnvVar("ONEDRIVE_CLIENT_SECRET"))?;
        let refresh_token = std::env::var("ONEDRIVE_REFRESH_TOKEN")
            .map_err(|_| AppStateError::MissingEnvVar("ONEDRIVE_REFRESH_TOKEN"))?;
        Ok(Self {
            root_path,
            refresh_token,
            client_id,
            client_secret,
            access_token: Arc::new(Mutex::new(None)),
        })
    }
}
