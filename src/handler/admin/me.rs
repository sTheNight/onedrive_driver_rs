use axum::{
    Json,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use sea_orm::{EntityTrait, PaginatorTrait};
use serde::Serialize;

use crate::{
    entity::admin_user, error::ErrorMessage, handler::admin::auth::ensure_admin_authenticated,
    state::AppState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUser {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub initialized: bool,
    pub user: CurrentUser,
}

pub async fn get_current_user(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    jar: CookieJar,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
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

    if existing_admin_user_count == 0 {
        return Err(ErrorMessage::new(
            StatusCode::CONFLICT,
            request_path,
            "admin user is not initialized",
        ));
    }

    let current_user = ensure_admin_authenticated(&jar, &request_path)?;

    Ok(Json(Response {
        initialized: true,
        user: CurrentUser {
            id: current_user.id,
            username: current_user.username,
        },
    }))
}
