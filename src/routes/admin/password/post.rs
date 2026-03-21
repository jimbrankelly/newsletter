//! src/routes/admin/password/post.rs
use actix_web::{HttpResponse, web, }; //error::InternalError, };
use actix_web_flash_messages::FlashMessage;
use secrecy::{SecretString, ExposeSecret, };
use sqlx::PgPool;
//use uuid::Uuid;

//use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use crate::routes::admin::dashboard::get_username;
use crate::authentication::{
    AuthError, Credentials, validate_credentials,
    UserId,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

/*async fn reject_anonymous_users(
    session: TypedSession
) -> Result<Uuid, actix_web::Error> {
    match session.get_user_id().map_err(e500)? {
        Some(user_id) => Ok(user_id),
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        }
    }
}*/

pub async fn change_password(
    form: web::Form<FormData>,
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    /*let user_id = session.get_user_id().map_err(e500)?;
    let Some(user_id) = user_id else {
        return Ok(see_other("/login"));
    };*/

    // new password must meet length requirements
    let password_len = form.new_password.expose_secret().len();
    if password_len < 13 {
        FlashMessage::error(
            "You entered a password that is too short.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }
    if password_len > 127 {
        FlashMessage::error(
            "You entered a password that is too long.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    // new password and check must match.  Note: secretstring doesn't implement eq.
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(*user_id, &pool).await.map_err(e500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e).into()),
        }
    }

    crate::authentication::change_password(*user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
