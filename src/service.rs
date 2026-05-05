use crate::{
    entity::onedrive_config,
    error::{OneDriveApiError, ServiceError},
    models::{self, FileListItem, GraphDriveItem, GraphListResponse},
    state::{AccessToken, AppState},
    utils,
};
use sea_orm::EntityTrait;
use std::collections::HashMap;

struct OneDriveConfig {
    root_path: String,
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

pub struct OneDriveApiService {
    pub state: AppState,
    client: reqwest::Client,
}

impl OneDriveApiService {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            state: state.clone(),
            client: state.http_client.clone(),
        }
    }

    pub async fn get_file_list(&self, path: &str) -> Result<Vec<FileListItem>, OneDriveApiError> {
        let config = self.get_onedrive_config().await?;
        let access_token = self.get_access_token(&config).await?;
        let url = utils::graph_children_url(&config.root_path, path)?;

        let response = self
            .client
            .get(url)
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
        let config = self.get_onedrive_config().await?;
        let url = utils::graph_item_url(&config.root_path, path)?;
        let token = self.get_access_token(&config).await?;
        let response = self.client.get(url).bearer_auth(token).send().await?;
        let response = Self::ensure_success(response).await?;

        Ok(FileListItem::from(response.json::<GraphDriveItem>().await?))
    }

    async fn get_onedrive_config(&self) -> Result<OneDriveConfig, ServiceError> {
        let config = onedrive_config::Entity::find_by_id(1)
            .one(&self.state.db_connection)
            .await?
            .ok_or(ServiceError::MissingOneDriveConfig)?;

        let client_id = non_empty_config_field(config.onedrive_client_id, "ONEDRIVE_CLIENT_ID")?;
        let client_secret =
            non_empty_config_field(config.onedrive_client_secret, "ONEDRIVE_CLIENT_SECRET")?;
        let refresh_token =
            non_empty_config_field(config.onedrive_refresh_token, "ONEDRIVE_REFRESH_TOKEN")?;

        Ok(OneDriveConfig {
            root_path: config.onedrive_root_path,
            client_id,
            client_secret,
            refresh_token,
        })
    }

    async fn get_access_token(&self, config: &OneDriveConfig) -> Result<String, OneDriveApiError> {
        {
            let token = self.state.access_token.read().await;
            if let Some(token) = token.as_ref()
                && !token.is_expired()
            {
                return Ok(token.access_token.clone());
            }
        }

        let mut params = HashMap::new();

        params.insert("client_id", config.client_id.clone());
        params.insert("client_secret", config.client_secret.clone());
        params.insert("refresh_token", config.refresh_token.clone());
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
        *self.state.access_token.write().await = Some(token);

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

fn non_empty_config_field(value: String, field_name: &'static str) -> Result<String, ServiceError> {
    if value.trim().is_empty() {
        Err(ServiceError::MissingOneDriveConfigField(field_name))
    } else {
        Ok(value)
    }
}
