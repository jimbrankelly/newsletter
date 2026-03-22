use actix_web::{
    HttpResponse,
    web, 
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    email_client::EmailClient,
    routes::get_confirmed_subscribers,
    utils::{see_other, e500},
};

#[derive(serde::Deserialize)]
pub struct NewsletterData {
    pub title: String,
    pub content_html: String,
    pub content_text: String,
}


pub async fn publish_newsletter_admin(
    form: web::Form<NewsletterData>,
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let _user_id = user_id.into_inner();

    if form.title.trim().is_empty() {
        FlashMessage::error(
            "The newsletter title is empty.",
        )
        .send();
        return Ok(see_other("/admin/newsletters"));
    }

    if form.content_html.trim().is_empty() {
        FlashMessage::error(
            "The newsletter html content is empty.",
        )
        .send();
        return Ok(see_other("/admin/newsletters"));
    }

    if form.content_text.trim().is_empty() {
        FlashMessage::error(
            "The newsletter plain text content is empty.",
        )
        .send();
        return Ok(see_other("/admin/newsletters"));
    }

    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .map_err(e500)?;

    for subscriber in subscribers {
        // The compiler forces us to handle both the happy and unhappy case!
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &form.title,
                    &form.content_html,
                    &form.content_text,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter issue to {}", subscriber.email)
                })
                .map_err(e500)?,
            Err(e) => tracing::warn!(
                    // We record the error chain as a structured field 
                    // on the log record.
                    error.cause_chain = ?e,
                    // Using `\` to split a long string literal over
                    // two lines, without creating a `\n` character.
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                ),
        };

    }

    FlashMessage::error("Your newsletter has been published.").send();
    Ok(see_other("/admin/newsletters"))
}
