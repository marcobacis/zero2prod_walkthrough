use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;
    app.login_with_test_user().await;

    let response = app.logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_page = app.get_login().await;
    assert!(html_page.contains("You have successfully logged out."));

    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
