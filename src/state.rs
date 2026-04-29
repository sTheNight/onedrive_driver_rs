use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("{0} is empty")]
    MissingEnvVar(&'static str),
    #[error("failed to build http client: {0}")]
    HttpClientBuild(#[from] reqwest::Error),
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
    pub http_client: reqwest::Client,
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
        let http_client = reqwest::Client::builder()
            .user_agent("onedrive_driver_rs/0.1")
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            root_path,
            refresh_token,
            client_id,
            client_secret,
            access_token: Arc::new(Mutex::new(None)),
            http_client,
        })
    }
}
