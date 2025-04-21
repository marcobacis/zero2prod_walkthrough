use std::fmt::Debug;

use actix_web::{
    http::header::{ContentType, LOCATION},
    web, HttpResponse,
};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{session_state::TypedSession, utils::e500};

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    let content = include_str!("./dashboard.html").replace("{username}", &username);
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(content))
}

pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username 
        FROM users 
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to retrieve username")?;
    Ok(row.username)
}
