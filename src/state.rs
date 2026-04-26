#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("{0} is empty")]
    MissingEnvVar(&'static str),
    #[error("{0} is null")]
    EmptyEnvVar(&'static str),
}
#[derive(Clone, Debug)]
pub struct AppState {
    pub root_path: String,
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: String,
}

impl AppState {
    pub fn from_env() -> Result<Self, AppStateError> {
        let root_path = std::env::var("ONEDRIVE_ROOT_PATH").unwrap();
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
        })
    }
}
