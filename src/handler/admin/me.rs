use axum::{
    Json,
    extract::{OriginalUri, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use serde::Serialize;

use crate::{
    error::ErrorMessage, handler::admin::auth::ensure_admin_authenticated, state::AppState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: i32,
    pub username: String,
}

pub async fn get_current_user(
    State(_state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    jar: CookieJar,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
    let current_user = ensure_admin_authenticated(&jar, &request_path)?;

    Ok(Json(Response {
        id: current_user.id,
        username: current_user.username,
    }))
}
