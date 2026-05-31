use axum::{
    Json,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use bcrypt::verify;
use jsonwebtoken::{EncodingKey, Header, encode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{entity::admin_user, error::ErrorMessage, state::AppState};

use super::auth::{ADMIN_TOKEN_COOKIE_NAME, jwt_secret};

const JWT_EXPIRES_IN_SECONDS: u64 = 30 * 60;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: i32,
    pub username: String,
    pub access_token: String,
    pub exp_min: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
}

pub async fn login(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    jar: CookieJar,
    Json(payload): Json<Request>,
) -> Result<impl IntoResponse, ErrorMessage> {
    let request_path = uri.path().to_string();
    let username = payload.username.trim().to_owned();

    if username.is_empty() || payload.password.is_empty() {
        return Err(invalid_credentials(request_path));
    }

    let admin_user = admin_user::Entity::find()
        .filter(admin_user::Column::Username.eq(username))
        .one(&state.db_connection)
        .await
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to query admin user: {err}"),
            )
        })?
        .ok_or_else(|| invalid_credentials(request_path.clone()))?;

    let password = payload.password;
    let password_hash = admin_user.password_hash.clone();
    let password_verified = tokio::task::spawn_blocking(move || verify(password, &password_hash))
        .await
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to join password verify task: {err}"),
            )
        })?
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                &request_path,
                format!("failed to verify password: {err}"),
            )
        })?;

    if !password_verified {
        return Err(invalid_credentials(request_path));
    }

    let token = create_admin_token(&admin_user, &request_path)?;
    let cookie = Cookie::build((ADMIN_TOKEN_COOKIE_NAME, token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(Response {
            id: admin_user.id,
            username: admin_user.username,
            access_token: token.clone(),
            exp_min: JWT_EXPIRES_IN_SECONDS,
        }),
    ))
}

fn create_admin_token(
    admin_user: &admin_user::Model,
    request_path: &str,
) -> Result<String, ErrorMessage> {
    let jwt_secret = jwt_secret(request_path)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| {
            ErrorMessage::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_path,
                format!("failed to read system time: {err}"),
            )
        })?
        .as_secs();

    let claims = Claims {
        sub: admin_user.id.to_string(),
        username: admin_user.username.clone(),
        exp: (now + JWT_EXPIRES_IN_SECONDS) as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|err| {
        ErrorMessage::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            request_path,
            format!("failed to create admin token: {err}"),
        )
    })
}

fn invalid_credentials(request_path: String) -> ErrorMessage {
    ErrorMessage::new(
        StatusCode::UNAUTHORIZED,
        request_path,
        "invalid username or password",
    )
}
