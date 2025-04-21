use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    // Fail login - receive redirect with cookie
    let login_body = serde_json::json!({
        "username": "not-existing-username",
        "password" : "wrong-password"
    });

    let form_response = app.post_login(&login_body).await;
    assert_is_redirect_to(&form_response, "/login");

    // Follow the redirect - should show the error
    let login_page = app.get_login().await;
    assert!(login_page.contains("<p>Authentication failed</p>"));

    // Get login page again - should not show the error
    let login_page = app.get_login().await;
    assert!(!login_page.contains("<p>Authentication failed</p>"));
}

#[tokio::test]
async fn redirect_to_dashboard_after_login_success() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password" : &app.test_user.password
    });

    let login_response = app.post_login(&login_body).await;
    assert_is_redirect_to(&login_response, "/admin/dashboard");

    let dashboard_page = app.get_admin_dashboard_html().await;
    assert!(dashboard_page.contains(&format!("Welcome {}", app.test_user.username)));
}

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}