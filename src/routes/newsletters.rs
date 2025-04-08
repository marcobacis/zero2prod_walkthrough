use actix_web::{
    http::{
        header::{self, HeaderMap, HeaderValue},
        StatusCode,
    },
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, telemetry::spawn_blocking_with_tracing,
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Unauthorized")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    name = "Publishing newsletter",
    skip(body, pool, email_client),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    request: HttpRequest,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &credentials.username);

    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    // We record the error chain as a structured field
                    // on the log record.
                    error.cause_chain= ?error,
                    // Using `\` to split a long string literal over
                    // two lines, without creating a `\n` character.
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("'Authorization' header missing")?
        .to_str()
        .context("'Authorization header is not a valid UTF-8 string'")?;

    let credentials_segment = header_value
        .strip_prefix("Basic ")
        .context("Given authentication is not Basic")?;

    let decoded_segment = base64::engine::general_purpose::STANDARD
        .decode(credentials_segment)
        .context("Failed to decode credentials segment from base64")?;

    let decoded_bytes = String::from_utf8(decoded_segment)
        .context("credentials segment is not a valid UTF8 string")?;

    let mut credentials = decoded_bytes.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("username not provided"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("password not provided"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    // Dummy hash to avoid timing attacks
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(PublishError::AuthError)?
    {
        user_id = Some(stored_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(credentials.password, expected_password_hash)
    })
    .await
    .context("Failed to spawn blocking task")
    .map_err(PublishError::UnexpectedError)?
    .context("Invalid password")
    .map_err(PublishError::AuthError)?;

    // user_id is set to Some only if a username matched
    // even if it matches the dummy password, we need to return error if the username wasn't found
    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username")))
}

fn verify_password_hash(
    password_candidate: Secret<String>,
    expected_password_hash: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash PHC string")
        .map_err(PublishError::UnexpectedError)?;

    tracing::info_span!("Verify password hash")
        .in_scope(|| {
            Argon2::default().verify_password(
                password_candidate.expose_secret().as_bytes(),
                &expected_password_hash,
            )
        })
        .context("Invalid password")
        .map_err(PublishError::AuthError)?;

    Ok(())
}

#[tracing::instrument(name = "Getting user credentials", skip(pool), fields(username))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform credentials validation query")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}

#[tracing::instrument(name = "Getting confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| match SubscriberEmail::parse(&r.email) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(e) => Err(anyhow::anyhow!(e)),
            })
            .collect();

    Ok(confirmed_subscribers)
}
