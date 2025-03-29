use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;


#[derive(serde::Deserialize)]
pub struct Parameters {
    pub token: String
}

#[tracing::instrument(name = "Confirming a pending subscriber", skip(_parameters))]
pub async fn confirm(_parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {

    let subscriber_id = match get_subscriber_id_from_token(&pool, &_parameters.token).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    match subscriber_id {
        None => HttpResponse::NotFound().finish(),
        Some(id) => {
            if(confirm_subscriber(&pool, id)).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(name = "Finding subscriber using token", skip(pool, token))]
async fn get_subscriber_id_from_token(pool: &PgPool, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        token
    ).fetch_optional(pool).await.map_err(|e| {
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
        tracing::error!("Failed to execute query: {}",e);
        e
    })?;
    Ok(())
}