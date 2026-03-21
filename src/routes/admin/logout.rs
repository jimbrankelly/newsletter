//! src/routes/admin/logout.rs
use crate::{
    authentication::UserId,
    session_state::TypedSession,
    utils::see_other,
};

use actix_web::{HttpResponse, web, };
use actix_web_flash_messages::FlashMessage;

pub async fn log_out(
    session: TypedSession,
    _user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    /*if session.get_user_id().map_err(e500)?.is_none() {
        Ok(see_other("/login"))
    } else {*/
        session.log_out();
        FlashMessage::info("You have successfully logged out.").send();
        Ok(see_other("/login"))
    //}
}
