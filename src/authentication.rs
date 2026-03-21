//! src/authentication.rs

use anyhow::{anyhow, Context, };
use argon2::{Argon2, PasswordHash, PasswordVerifier, };
use secrecy::{SecretString, ExposeSecret, };
use sqlx::PgPool;

use crate::{
    routes::PublishError,
    telemetry::spawn_blocking_with_tracing,
};

 
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    /*let (user_id, expected_password_hash) = get_stored_credentials(
            &credentials.username, 
            &pool
        )
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))?
        ;*/
    let mut user_id = None;
    let mut expected_password_hash = SecretString::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .into()
    );
    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, &pool)
            .await
            .map_err(AuthError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    // This executes before spawning the new thread
    //let current_span = tracing::Span::current();
    //let _ =tokio::task::spawn_blocking(move || {
    spawn_blocking_with_tracing(move || {
        verify_password_hash(
            expected_password_hash, 
            credentials.password
        )
    })
    .await
    // spawn_blocking is fallible - we have a nested Result here!
    .context("Failed to spawn blocking task.")?
    .map_err(|_| AuthError::InvalidCredentials(anyhow!("Invalid password.")))?;

    //Ok(user_id)

    // This is only set to `Some` if we found credentials in the store
    // So, even if the default password ends up matching (somehow)
    // with the provided password, 
    // we never authenticate a non-existing user.
    // You can easily add a unit test for that precise scenario.
    user_id.ok_or_else(|| 
        AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username."))
    )
}

#[tracing::instrument(
    name = "Verify password hash", 
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(
            expected_password_hash.expose_secret()
        )
        .map_err(|_| PublishError::AuthError( anyhow!(
            "Failed to parse hash in PHC string format."
        )))?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash
        )
        //.context("Invalid password.")
        .map_err(|_| PublishError::AuthError( anyhow!("Invalid password.")))
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(username: &str, pool: &PgPool) 
    -> Result<Option<(uuid::Uuid, SecretString)>, anyhow::Error> 
{
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, SecretString::new(row.password_hash.into())));
    Ok(row)
}

