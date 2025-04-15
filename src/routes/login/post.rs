
use actix_web::{
    error::InternalError,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse, ResponseError,
};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    startup::HmacSecret,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Authentication error")]
    AuthError(#[source] anyhow::Error),
    #[error("Unexpected error")]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
}

#[tracing::instrument(
    "Login Form Post",
    skip(form, pool, hmac_secret),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    hmac_secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })
        .map_err(|e| redirect_with_error("/login", e, &hmac_secret))?;

    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}

fn redirect_with_error<E: ToString>(to: &str, e: E, hmac_secret: &HmacSecret) -> InternalError<E> {
    let encoded_error = urlencoding::Encoded::new(e.to_string());
    let query_params = format!("error={}", encoded_error);

    let secret: &[u8] = hmac_secret.0.expose_secret().as_bytes();
    let hmac_tag = {
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
        mac.update(query_params.as_bytes());
        mac.finalize().into_bytes()
    };

    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, format!("{to}?{query_params}&tag={hmac_tag:x}")))
        .finish();

    InternalError::from_response(e, response)
}
