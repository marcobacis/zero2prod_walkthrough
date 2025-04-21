use std::fmt::{Debug, Display};

use actix_web::{http::header::{ContentType, LOCATION}, web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(opaque500)? {
        get_username(user_id, &pool).await.map_err(opaque500)?
    } else {
        return Ok(HttpResponse::SeeOther().insert_header((LOCATION, "/login")).finish());
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
</body>
</html>"#
        )))
}

async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
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

fn opaque500<TErr>(e: TErr) -> actix_web::Error
where
    TErr: Debug + Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}
