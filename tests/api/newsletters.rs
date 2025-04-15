use tokio::test;
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

#[test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;

    let test_cases = vec![
        (
            serde_json::json!({"content": {"text":"plain text", "html": "html text"}}),
            "missing title",
        ),
        (serde_json::json!({"title": "example"}), "missing content"),
        (
            serde_json::json!({"title": "example", "content": {"html": "html text"}}),
            "missing plain text",
        ),
        (
            serde_json::json!({"title": "example", "content": {"text":"plain text"}}),
            "missing html content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "/newsletters didn't fail with 400 when the message had {}",
            error_message
        );
    }
}

#[test]
async fn newsletter_is_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    // We expect not mails to be sent
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletters(&dummy_newsletter_body()).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // We expect 1 mail to be sent
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletters(&dummy_newsletter_body()).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .json(&dummy_newsletter_body())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;

    let random_user = Uuid::new_v4().to_string();
    let random_psw = Uuid::new_v4().to_string();

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .basic_auth(random_user, Some(random_psw))
        .json(&dummy_newsletter_body())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[test]
async fn invalid_password_is_rejected() {
    let app = spawn_app().await;

    let username = &app.test_user.username;
    let random_psw = Uuid::new_v4().to_string();

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(random_psw))
        .json(&dummy_newsletter_body())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

fn dummy_newsletter_body() -> serde_json::Value {
    serde_json::json!({
        "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    }
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
