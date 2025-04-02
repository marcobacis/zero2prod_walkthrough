
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};


// Input data
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(&value.name)?;
        let email = SubscriberEmail::parse(&value.email)?;
        Ok(Self { email, name })
    }
}

// Errors
#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Adding a new subscriber", skip(form, pool, email_client, base_url),fields(subscriber_email = %form.email, subscriber_name = %form.name))]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = pool.begin().await.context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber).await.context("Failed to insert new subscriber in the database.")?;
    let token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &token).await.context("Failed to store the confirmation token for a new subscriber.")?;
    send_confirmation_email(&email_client, new_subscriber, &base_url.0, &token).await.context("Failed to commit SQL transaction to store a new subscriber.")?;
    transaction.commit().await.context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction.as_mut())
    .await?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the databsasae",
    skip(transaction, token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES($1, $2)"#,
        token,
        subscriber_id
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Sending confirmation email to subscriber",
    skip(email_client, new_subscriber, base_url)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?token={}", base_url, token);
    let html_body = format!(
        "Welcome to the newsletter!<br/>\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let text_body = format!(
        "Welcome to the newsletter!\n\
        Visit {} to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome", &html_body, &text_body)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}