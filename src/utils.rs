use std::fmt::{Debug, Display};

use actix_web::{error::InternalError, http::header::LOCATION, HttpResponse};
use actix_web_flash_messages::FlashMessage;

pub fn e500<TErr>(e: TErr) -> actix_web::Error
where
    TErr: Debug + Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn e400<TErr>(e: TErr) -> actix_web::Error
where
    TErr: Debug + Display + 'static,
{
    actix_web::error::ErrorBadRequest(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}

pub fn redirect_with_error<E: ToString>(to: &str, e: E) -> InternalError<E> {
    FlashMessage::error(e.to_string()).send();
    let response: HttpResponse = HttpResponse::SeeOther()
        .insert_header((LOCATION, to.to_string()))
        .finish();

    InternalError::from_response(e, response)
}
