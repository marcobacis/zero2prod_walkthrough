use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    pub token: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("No subscriber associated with the given token.")]
    TokenNotFound,
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ConfirmationError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmationError::TokenNotFound => actix_web::http::StatusCode::NOT_FOUND,
        }
    }
}

#[tracing::instrument(name = "Confirming a pending subscriber", skip(_parameters))]
pub async fn confirm(_parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> Result<HttpResponse, ConfirmationError> {
    let subscriber_id = get_subscriber_id_from_token(&pool, &_parameters.token)
        .await
        .context("Failed to retrieve the subscriber associated with the given token")?
        .ok_or(ConfirmationError::TokenNotFound)?;

    confirm_subscriber(&pool, subscriber_id)
        .await
        .context("Failed to update the subscriber status to 'confirmed'")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Finding subscriber using token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {}", e);
        e
    })?;
    Ok(())
}
