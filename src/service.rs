use crate::{
    models::{self, FileListItem, GraphListResponse},
    state::AppState,
};
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

#[derive(Debug, thiserror::Error)]
pub enum OneDriveApiError {
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("invalid expires_in value: {0}")]
    InvalidExpiresIn(i64),
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_at: Instant,
}

impl AccessToken {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

pub struct OneDriveApiService {
    pub state: AppState,
    client: reqwest::Client,
    access_token: Option<AccessToken>,
}

impl OneDriveApiService {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            state: state.clone(),
            client: reqwest::Client::builder()
                .user_agent("onedrive_driver_rs/0.1")
                .timeout(Duration::from_secs(60))
                .build()
                .expect("failed to build reqwest client"),
            access_token: None,
        }
    }

    pub async fn get_file_list(&mut self, path: &str) -> Vec<FileListItem> {
        let access_token = self.get_access_token().await.unwrap();

        let path = path.trim_matches('/');

        let url = if path.is_empty() {
            "https://graph.microsoft.com/v1.0/me/drive/root/children".to_string()
        } else {
            format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:/{}:/children",
                path
            )
        };

        let response = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .unwrap();

        let response_deser = response.json::<GraphListResponse>().await.unwrap();

        response_deser
            .value
            .into_iter()
            .map(FileListItem::from)
            .collect()
    }

    async fn get_access_token(&mut self) -> Result<String, OneDriveApiError> {
        if let Some(token) = &self.access_token {
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }
        }

        let mut params = HashMap::new();

        params.insert("client_id", self.state.client_id.clone());
        params.insert("client_secret", self.state.client_secret.clone());
        params.insert("refresh_token", self.state.refresh_token.clone());
        params.insert("grant_type", "refresh_token".to_string());

        let response = self
            .client
            .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| OneDriveApiError::RequestFailed(e.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            return Err(OneDriveApiError::RequestFailed(format!(
                "status: {status}, body: {body}"
            )));
        }

        let token_response = response
            .json::<models::TokenResponse>()
            .await
            .map_err(|e| OneDriveApiError::RequestFailed(e.to_string()))?;

        if token_response.expires_in <= 0 {
            return Err(OneDriveApiError::InvalidExpiresIn(
                token_response.expires_in,
            ));
        }

        let expires_in = token_response.expires_in as u64;
        let safe_expires_in = expires_in.saturating_sub(60);

        let token = AccessToken {
            access_token: token_response.access_token,
            expires_at: Instant::now() + Duration::from_secs(safe_expires_in),
        };

        let access_token = token.access_token.clone();
        self.access_token = Some(token);

        Ok(access_token)
    }
}
