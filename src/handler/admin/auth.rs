use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{DecodingKey, Validation, decode};

use crate::error::ErrorMessage;

use super::login::Claims;

pub(crate) const ADMIN_TOKEN_COOKIE_NAME: &str = "admin_token";

pub(crate) fn ensure_admin_authenticated(
    jar: &CookieJar,
    request_path: &str,
) -> Result<(), ErrorMessage> {
    let token = jar
        .get(ADMIN_TOKEN_COOKIE_NAME)
        .map(|cookie| cookie.value())
        .ok_or_else(|| unauthorized(request_path))?;
    let jwt_secret = jwt_secret(request_path)?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|_| ())
    .map_err(|_| unauthorized(request_path))
}

pub(crate) fn jwt_secret(request_path: &str) -> Result<String, ErrorMessage> {
    std::env::var("JWT_SECRET").map_err(|_| {
        ErrorMessage::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            request_path,
            "JWT_SECRET is missing",
        )
    })
}

fn unauthorized(request_path: &str) -> ErrorMessage {
    ErrorMessage::new(
        StatusCode::UNAUTHORIZED,
        request_path,
        "admin authentication required",
    )
}
