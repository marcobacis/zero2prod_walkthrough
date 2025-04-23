use actix_web::{
    web::{self, ReqData},
    HttpResponse,
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publishing newsletter",
    skip(form, pool),
    fields(user_id=%&*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<BodyData>,
    pool: web::Data<PgPool>,
    user_id: ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let BodyData {
        title,
        html_content,
        text_content,
        idempotency_key,
    } = form.0;

    // Make call idempotent
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(http_response) => {
            success_message().send();
            return Ok(http_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("failed to store newsletter issue details")
        .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;

    success_message().send();

    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted!")
}

#[tracing::instrument(
    name = "Creating newsletter issue",
    skip(transaction, text_content, html_content)
)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'static, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, anyhow::Error> {
    let newsletter_issue_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues
        (newsletter_issue_id,title,text_content,html_content,published_at)
        VALUES ($1, $2, $3, $4, now());
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(name = "Enqueue delivery tasks for newsletter", skip(transaction))]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (newsletter_issue_id, subscriber_email)
        SELECT $1, email FROM subscriptions WHERE status = 'confirmed';
        "#,
        newsletter_issue_id
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(())
}
