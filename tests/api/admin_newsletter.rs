//! tests/api/admin_newsletter.rs
use crate::helpers::{
    ConfirmationLinks,
    spawn_app, assert_is_redirect_to, TestApp, 
};
use secrecy::ExposeSecret;
use uuid::Uuid;

use wiremock::{
    Mock, ResponseTemplate, 
    matchers::{any, path, method, }, 
};

use newsletter::routes::NewsletterData;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_publish_newsletter_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_publish_newsletter().await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_newsletter() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .post_publish_newsletter(&serde_json::json!({
            "title": "".to_string(),
            "content_html": Uuid::new_v4().to_string(),
            "content_text": Uuid::new_v4().to_string(),
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_newsletter_data_must_not_be_empty() {
    // Arrange
    let app = spawn_app().await;
    //let new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password.expose_secret()
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act - Part 2 - Try to publish newsletter
    let test_cases = vec![
        (
            NewsletterData {
                title: "".to_string(),
                content_text: "Newsletter body as plain text".to_string(),
                content_html: "<p>Newsletter body as HTML</p>".to_string(),
            },
            "<p><i>The newsletter title is empty.</i></p>",
        ),
        (
            NewsletterData {
                title: "a title".to_string(),
                content_text: "".to_string(),
                content_html: "<p>Newsletter body as HTML</p>".to_string(),
            },
            "<p><i>The newsletter plain text content is empty.</i></p>",
        ),
        (
            NewsletterData {
                title: "a title".to_string(),
                content_text: "plain text content".to_string(),
                content_html: "".to_string(),
            },
            "<p><i>The newsletter html content is empty.</i></p>",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app
            .post_publish_newsletter(&serde_json::json!({
                "title": invalid_body.title,
                "content_html": invalid_body.content_html,
                "content_text": invalid_body.content_text,
            }))
            .await;

        // Assert
        assert_is_redirect_to(&response, "/admin/newsletters");

        // Act - Part 3 - Follow the redirect
        let html_page = app.get_publish_newsletter_html().await;
        dbg!(error_message);
        assert!(html_page.contains( error_message ));
    
    }


}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark!
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password.expose_secret()
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // A sketch of the newsletter payload structure.
    // We might change it later on.
    let response = app.post_publish_newsletter(&serde_json::json!({
                "title": "Newsletter title",
                "content_html": "<p>Newsletter body as HTML</p>",
                "content_text": "Newsletter body as plain text",
            }))
            .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
    // Mock verifies on Drop that we haven't sent the newsletter email 
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password.expose_secret()
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // A sketch of the newsletter payload structure.
    // We might change it later on.
    let response = app.post_publish_newsletter(&serde_json::json!({
                "title": "Newsletter title",
                "content_html": "<p>Newsletter body as HTML</p>",
                "content_text": "Newsletter body as plain text",
            }))
            .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
    // Mock verifies on Drop that we have sent the newsletter email
}

/// Use the public API of the application under test to create
/// an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    // We now inspect the requests received by the mock Postmark server
    // to retrieve the confirmation link and return it 
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    // We can then reuse the same helper and just add 
    // an extra step to actually call the confirmation link!
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
