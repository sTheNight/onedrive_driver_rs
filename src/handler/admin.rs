use axum::{
    Json,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use bcrypt::{DEFAULT_COST, hash, verify};
use jsonwebtoken::{EncodingKey, Header, encode};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{entity::admin_user, error::ErrorMessage, state::AppState};

const ADMIN_TOKEN_COOKIE_NAME: &str = "admin_token";
const JWT_EXPIRES_IN_SECONDS: u64 = 7 * 24 * 60 * 60;

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

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Serialize)]
struct AdminClaims {
    sub: String,
    username: String,
    exp: usize,
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

pub async fn login(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
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
    let cookie = Cookie::build((ADMIN_TOKEN_COOKIE_NAME, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(LoginResponse {
            id: admin_user.id,
            username: admin_user.username,
        }),
    ))
}

fn create_admin_token(
    admin_user: &admin_user::Model,
    request_path: &str,
) -> Result<String, ErrorMessage> {
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| {
        ErrorMessage::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            request_path,
            "JWT_SECRET is missing",
        )
    })?;

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

    let claims = AdminClaims {
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
