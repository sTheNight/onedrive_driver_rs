use super::auth::ensure_admin_authenticated;
use crate::{entity::onedrive_config, error::ErrorMessage, state::AppState};
use axum::{
    Json,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub one_drive_root_path: String,
    pub one_drive_client_id: String,
    pub one_drive_client_secret: String,
    pub one_drive_refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: i32,
    pub one_drive_root_path: String,
    pub one_drive_client_id: String,
    pub one_drive_client_secret: String,
    pub one_drive_refresh_token: String,
}

pub async fn get_onedrive_config(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    jar: CookieJar,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
    let _ = ensure_admin_authenticated(&jar, &request_path)?;
    let config = onedrive_config::Entity::find_by_id(1)
        .one(&state.db_connection)
        .await
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to query onedrive config: {err}"),
            )
        })?;
    match config {
        None => Err(ErrorMessage::new(
            StatusCode::CONFLICT,
            &request_path,
            "onedrive config is not initialized",
        )),
        Some(config) => Ok(Json(Response::from(config))),
    }
}

pub async fn update_onedrive_config(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    jar: CookieJar,
    Json(payload): Json<Request>,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
    let _ = ensure_admin_authenticated(&jar, &request_path)?;

    let existing_config = onedrive_config::Entity::find_by_id(1)
        .one(&state.db_connection)
        .await
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to query onedrive config: {err}"),
            )
        })?;

    let active_model = onedrive_config::ActiveModel {
        id: Set(1),
        onedrive_root_path: Set(payload.one_drive_root_path),
        onedrive_client_id: Set(payload.one_drive_client_id),
        onedrive_client_secret: Set(payload.one_drive_client_secret),
        onedrive_refresh_token: Set(payload.one_drive_refresh_token),
    };

    let config = match existing_config {
        Some(_) => active_model.update(&state.db_connection).await,
        None => active_model.insert(&state.db_connection).await,
    }
    .map_err(|err| {
        ErrorMessage::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            &request_path,
            format!("failed to save onedrive config: {err}"),
        )
    })?;

    *state.access_token.write().await = None;
    *state.onedrive_config.write().await = None;

    Ok(Json(Response::from(config)))
}

impl From<onedrive_config::Model> for Response {
    fn from(config: onedrive_config::Model) -> Self {
        Self {
            id: config.id,
            one_drive_root_path: config.onedrive_root_path,
            one_drive_client_id: config.onedrive_client_id,
            one_drive_client_secret: config.onedrive_client_secret,
            one_drive_refresh_token: config.onedrive_refresh_token,
        }
    }
}
