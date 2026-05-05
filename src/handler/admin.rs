use axum::{
    Json,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::IntoResponse,
};
use bcrypt::{DEFAULT_COST, hash};
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};
use serde::{Deserialize, Serialize};

use crate::{entity::admin_user, error::ErrorMessage, state::AppState};

#[derive(Debug, Deserialize)]
pub struct InitAdminRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitAdminResponse {
    pub id: i32,
    pub username: String,
}

pub async fn init_admin_user(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Json(payload): Json<InitAdminRequest>,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
    let username = payload.username.trim().to_owned();

    if username.is_empty() {
        return Err(ErrorMessage::new(
            StatusCode::BAD_REQUEST,
            request_path,
            "username is empty",
        ));
    }

    if payload.password.is_empty() {
        return Err(ErrorMessage::new(
            StatusCode::BAD_REQUEST,
            request_path,
            "password is empty",
        ));
    }

    let existing_admin_user_count = admin_user::Entity::find()
        .count(&state.db_connection)
        .await
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to query existing admin user count: {err}"),
            )
        })?;

    if existing_admin_user_count > 0 {
        return Err(ErrorMessage::new(
            StatusCode::CONFLICT,
            request_path,
            "admin user already initialized",
        ));
    }

    let password = payload.password;
    let password_hash = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
        .await
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to join password hash task: {err}"),
            )
        })?
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to hash password: {err}"),
            )
        })?;

    let admin_user = admin_user::ActiveModel {
        id: Set(1),
        username: Set(username),
        password_hash: Set(password_hash),
    }
    .insert(&state.db_connection)
    .await
    .map_err(|err| {
        let status = if err.to_string().contains("UNIQUE constraint failed") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        ErrorMessage::new(
            status,
            &request_path,
            format!("failed to create admin user: {err}"),
        )
    })?;

    Ok(Json(InitAdminResponse {
        id: admin_user.id,
        username: admin_user.username,
    }))
}
