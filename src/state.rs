use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::Instant};

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("failed to build http client: {0}")]
    HttpClientBuild(#[from] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_at: Instant,
}
#[derive(Clone, Debug)]
pub struct OneDriveConfig {
    pub root_path: String,
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
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
    pub access_token: Arc<RwLock<Option<AccessToken>>>,
    pub db_connection: DatabaseConnection,
    pub http_client: reqwest::Client,
    pub onedrive_config: Arc<RwLock<Option<OneDriveConfig>>>,
}

impl AppState {
    pub fn init(db_connection: DatabaseConnection) -> Result<Self, AppStateError> {
        let http_client = reqwest::Client::builder()
            .user_agent("onedrive_driver_rs/0.1")
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            access_token: Arc::new(RwLock::new(None)),
            db_connection,
            http_client,
            onedrive_config: Arc::new(RwLock::new(None)),
        })
    }
}
