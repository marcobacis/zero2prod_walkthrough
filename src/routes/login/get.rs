use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let messages_html: String = flash_messages
        .iter()
        .map(|m| format!("<p><i>{}</i></p>", m.content()))
        .collect();

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login.html").replace("{error}", &messages_html))
}
