use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn change_password_form(
    session: TypedSession,
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    }

    let mut error_msg = String::new();
    for m in messages.iter() {
        writeln!(error_msg, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let body = include_str!("./password_form.html").replace("{error}", &error_msg);

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body))
}
