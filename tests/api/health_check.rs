use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let TestApp { address: listen_addr, ..} = spawn_app().await;

    // We need to bring in `reqwest` 
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();

    // Act
    let response = client
            .get(&format!("{listen_addr}/health_check"))
            .send()
            .await
            .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
