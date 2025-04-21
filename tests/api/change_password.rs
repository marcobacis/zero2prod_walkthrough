use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_see_change_psw_form() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    app.login_with_test_user().await;

    let new_password = Uuid::new_v4().to_string();
    let new_password_check = Uuid::new_v4().to_string();

    // Non-matching passwords: redirect back with error
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": new_password,
            "new_password_check": new_password_check
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    app.login_with_test_user().await;

    let current_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": current_password,
            "new_password": new_password,
            "new_password_check": new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

// TODO Test password validation (length, symbols, numbers, upper/lower case etc..)

#[tokio::test]
async fn changing_password_works() {
    let app = spawn_app().await;
    app.login_with_test_user().await;

    // Password change is successful
    let new_password = Uuid::new_v4().to_string();
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": new_password,
            "new_password_check": new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    let change_psw_html = app.get_change_password_html().await;
    assert!(change_psw_html.contains("Your password has been changed"));

    // After logout, the user can log in with the new password
    app.logout().await;

    let response = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");
}
