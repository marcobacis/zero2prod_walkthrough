use reqwest::Url;
use wiremock::{matchers::{method, path}, Mock, Respond, ResponseTemplate};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmation_without_token_is_rejected_with_400() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn link_returned_by_subscription_returns_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    // Act
    let response = reqwest::get(links.html).await.unwrap();
    // Assert
    assert_eq!(response.status().as_u16(), 200);
}
