use actix_web::{
    http::header::ContentType,
    web::{self},
    HttpResponse,
};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    pub fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let secret: &[u8] = secret.0.expose_secret().as_bytes();
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

pub async fn login_form(
    query: Option<web::Query<QueryParams>>,
    hmac_secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error = match query {
        Some(query) => match query.0.verify(&hmac_secret) {
            Ok(e) => format!("<p>{}</p>", htmlescape::encode_minimal(&e)),
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify the query parameters using the HMAC tag"
                );
                "".into()
            }
        },
        None => "".into(),
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login.html").replace("{error}", &error))
}
