use axum::{
    Json,
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderValue, header},
    response::IntoResponse,
};

use crate::{service::OneDriveApiService, state::AppState};

pub async fn get_file_list(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> impl IntoResponse {
    let path = uri.path().trim_start_matches("/api/list").trim_matches('/');
    let mut service = OneDriveApiService::from_state(&state);

    let list = service.get_file_list(path).await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    (headers, Json(list))
}
pub async fn download_file() -> impl IntoResponse {
    return Json("Hello World!");
}
