use std::time::Duration;

use tokio::test;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

#[test]
async fn you_must_be_logged_in_to_see_newsletter_form() {
    let app = spawn_app().await;
    let response = app.get_newsletter_form().await;
    assert_is_redirect_to(&response, "/login");
}

#[test]
async fn you_must_be_logged_in_to_send_newsletter() {
    let app = spawn_app().await;
    let response = app.post_newsletter(&dummy_newsletter_body()).await;
    assert_is_redirect_to(&response, "/login");
}

#[test]
async fn newsletter_is_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.login_with_test_user().await;
    create_unconfirmed_subscriber(&app).await;

    // We expect not mails to be sent
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletter(&dummy_newsletter_body()).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html = app.get_newsletter_form_html().await;
    assert!(html.contains("The newsletter issue has been published!"));
}

#[test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    app.login_with_test_user().await;
    create_confirmed_subscriber(&app).await;

    // We expect 1 mail to be sent
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletter(&dummy_newsletter_body()).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html = app.get_newsletter_form_html().await;
    assert!(html.contains("The newsletter issue has been published!"));
}

#[test]
async fn newsletter_delivery_is_idempotent() {
    let app = spawn_app().await;
    app.login_with_test_user().await;
    create_confirmed_subscriber(&app).await;

    // We expect 1 mail to be sent even with 2 requests
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = dummy_newsletter_body();

    // Request #1
    let response = app.post_newsletter(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletter_form_html().await;
    assert!(html_page.contains("The newsletter issue has been published"));

    // Request #2 with same body (even idempotency key)
    let response = app.post_newsletter(&body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletter_form_html().await;
    assert!(html_page.contains("The newsletter issue has been published"));
}

#[test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_with_test_user().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        // Setting a long delay to ensure that the second request
        // arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response1 = app.post_newsletter(&newsletter_request_body);
    let response2 = app.post_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);
    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
}

fn dummy_newsletter_body() -> serde_json::Value {
    serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    })
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
