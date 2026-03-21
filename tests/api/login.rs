//! tests/api/login.rs
use secrecy::ExposeSecret;

use crate::helpers::{spawn_app, assert_is_redirect_to, };
//use std::collections::HashSet;
//use reqwest::header::HeaderValue;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    // Assert
    assert_is_redirect_to(&response, "/login");

    //let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    //assert_eq!(flash_cookie.value(), "Authentication failed");

    /*let cookies: HashSet<_> = response
        .headers()
        .get_all("Set-Cookie")
        .into_iter()
        .collect();
    assert!(cookies
        .contains(&Header//Value::from_str("_flash=Authentication failed").unwrap())
    );*/
    // Act - Part 2
    let html_page = app.get_login_html().await;
    //assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    // Act - Part 3 - Reload the login page
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
    //assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

}


#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    // Arrange
    let app = spawn_app().await;

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password.expose_secret(),
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}
