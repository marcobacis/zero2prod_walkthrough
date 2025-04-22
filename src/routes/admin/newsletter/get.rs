use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn newsletter_form(
    messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut error_msg = String::new();
    for m in messages.iter() {
        writeln!(error_msg, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let body = include_str!("./newsletter_form.html").replace("{messages}", &error_msg);

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body))
}
