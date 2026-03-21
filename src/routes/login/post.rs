//! src/routes/login/post.rs
use actix_web::{
    error::InternalError,
    HttpResponse, web, 
    http::{header::LOCATION,  },
};
use hmac::{Hmac, Mac, };
use secrecy::{SecretString, ExposeSecret, };
use sqlx::PgPool;

use crate::{
    authentication::{ AuthError, Credentials, validate_credentials },
    routes::error_chain_fmt,
    startup::HmacSecret,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[tracing::instrument(
    skip(form, pool, secret),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>, 
    pool: web::Data<PgPool>,
    secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<LoginError>>  {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(_user_id) => {
            // [...]
            // We need to Ok-wrap again
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            // [...]
            let query_string = format!(
                "error={}", 
                urlencoding::Encoded::new(e.to_string())
            );
            // We need the secret here - how do we get it?
            //let secret: &[u8] = todo!();
            let hmac_tag = {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                    secret.0.expose_secret().as_bytes()
                ).unwrap();
                //let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };

            let response = HttpResponse::SeeOther()
                .insert_header((
                    LOCATION,
                    format!("/login?{}&tag={:x}", query_string, hmac_tag),
                ))
                .finish();
            Err(InternalError::from_response(e.into(), response))
        }
    }

    /*let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;*/
    /* {
        Ok(user_id) => {
            tracing::Span::current()
                .record("user_id", &tracing::field::display(&user_id));
            HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish()
        }
        Err(_) => {
            todo!()
        }
    }*/
    /*Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())*/
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl From<AuthError> for LoginError {
    fn from( e: AuthError) -> Self {
        match e {
            AuthError::InvalidCredentials(e) => LoginError::AuthError(e),
            AuthError::UnexpectedError(e) => LoginError::AuthError(e),
        }
    }
}

/*impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        //let encoded_error = urlencoding::Encoded::new(self.to_string());
        let query_string = format!(
            "error={}", 
            urlencoding::Encoded::new(self.to_string())
        );
        // We need the secret here - how do we get it?
        let secret: &[u8] = todo!();
        let hmac_tag = {
            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };
        HttpResponse::build(self.status_code())
            // Appending the hexadecimal representation of the HMAC tag to the 
            // query string as an additional query parameter.
            .insert_header((
                LOCATION, 
                format!("/login?{}&tag={:x}", query_string, hmac_tag)
            ))
            //.insert_header((LOCATION, format!("/login?error={}", encoded_error)))
            .finish()
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
}*/
