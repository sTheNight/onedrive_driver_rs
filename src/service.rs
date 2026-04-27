use crate::{
    error::OneDriveApiError,
    models::{self, FileListItem, GraphDriveItem, GraphListResponse},
    state::{AccessToken, AppState},
};
use std::{collections::HashMap, time::Duration};

pub struct OneDriveApiService {
    pub state: AppState,
    client: reqwest::Client,
}

impl OneDriveApiService {
    pub fn from_state(state: &AppState) -> Result<Self, OneDriveApiError> {
        Ok(Self {
            state: state.clone(),
            client: reqwest::Client::builder()
                .user_agent("onedrive_driver_rs/0.1")
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(OneDriveApiError::HttpClientBuild)?,
        })
    }

    pub async fn get_file_list(&self, path: &str) -> Result<Vec<FileListItem>, OneDriveApiError> {
        let access_token = self.get_access_token().await?;

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
            .await?;
        let response = Self::ensure_success(response).await?;

        let response_deser = response.json::<GraphListResponse>().await?;

        Ok(response_deser
            .value
            .into_iter()
            .map(FileListItem::from)
            .collect())
    }

    pub async fn get_item_info(&self, path: &str) -> Result<FileListItem, OneDriveApiError> {
        let url = format!("https://graph.microsoft.com/v1.0/me/drive/root:/{}:", path);
        let token = self.get_access_token().await?;
        let response = self.client.get(&url).bearer_auth(token).send().await?;
        let response = Self::ensure_success(response).await?;

        Ok(FileListItem::from(response.json::<GraphDriveItem>().await?))
    }

    async fn get_access_token(&self) -> Result<String, OneDriveApiError> {
        {
            let token = self.state.access_token.lock().await;
            if let Some(token) = token.as_ref() {
                if !token.is_expired() {
                    return Ok(token.access_token.clone());
                }
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
            .await?;
        let response = Self::ensure_success(response).await?;

        let token_response = response.json::<models::TokenResponse>().await?;

        if token_response.expires_in <= 0 {
            return Err(OneDriveApiError::InvalidExpiresIn(
                token_response.expires_in,
            ));
        }

        let token = AccessToken::new(
            token_response.access_token,
            token_response.expires_in as u64,
        );

        let access_token = token.access_token.clone();
        *self.state.access_token.lock().await = Some(token);

        Ok(access_token)
    }

    async fn ensure_success(
        response: reqwest::Response,
    ) -> Result<reqwest::Response, OneDriveApiError> {
        let status = response.status();

        match status {
            status if status.is_success() => Ok(response),
            status => {
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|err| format!("failed to read upstream error body: {err}"));

                Err(OneDriveApiError::UpstreamStatus {
                    status: status.as_u16(),
                    body,
                })
            }
        }
    }
}
