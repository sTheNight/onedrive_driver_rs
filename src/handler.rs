use crate::{
    error::ErrorMessage, models::FileListItemType, service::OneDriveApiService, state::AppState,
};
use axum::{
    Json,
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};

pub async fn get_file_list(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
    let path = request_path
        .trim_start_matches("/api/list")
        .trim_matches('/')
        .to_string();
    let service = OneDriveApiService::from_state(&state)
        .map_err(|err| ErrorMessage::from(err).with_request_path(&request_path))?;

    let list = service
        .get_file_list(&path)
        .await
        .map_err(|err| ErrorMessage::from(err).with_request_path(&request_path))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    Ok((headers, Json(list)))
}
// 这个 handler 的意义似乎不是很大
pub async fn download_file(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ErrorMessage> {
    let request_path = uri.path().to_string();
    let path = request_path
        .trim_start_matches("/api/download")
        .trim_matches('/')
        .to_string();

    let service = OneDriveApiService::from_state(&state)
        .map_err(|err| ErrorMessage::from(err).with_request_path(&request_path))?;
    let item = service
        .get_item_info(&path)
        .await
        .map_err(|err| ErrorMessage::from(err).with_request_path(&request_path))?;
    match item.item_type {
        FileListItemType::File => match item.download_url {
            Some(download_url) if !download_url.is_empty() => {
                Ok(Redirect::temporary(download_url.as_str()).into_response())
            }
            _ => Err(ErrorMessage::new(
                StatusCode::NOT_FOUND,
                request_path,
                "download url not found",
            )),
        },
        FileListItemType::Folder => Err(ErrorMessage::new(
            StatusCode::BAD_REQUEST,
            request_path,
            "folder cannot be downloaded directly",
        )),
    }
}
