use actix_web::{
    HttpResponse, web 
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    email_client::EmailClient,
    idempotency::{IdempotencyKey, NextAction, try_processing, save_response, },
    routes::get_confirmed_subscribers,
    utils::{see_other, e500, e400, },
};

#[derive(serde::Deserialize)]
pub struct NewsletterData {
    pub title: String,
    pub content_html: String,
    pub content_text: String,
    pub idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip( form, pool, email_client, ),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter_admin(
    form: web::Form<NewsletterData>,
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    // We must destructure the form to avoid upsetting the borrow-checker
    let NewsletterData { title, content_text, content_html, idempotency_key } = form.0;
     if title.trim().is_empty() {
        FlashMessage::error(
            "The newsletter title is empty.",
        )
        .send();
        return Ok(see_other("/admin/newsletters"));
    }

    if content_html.trim().is_empty() {
        FlashMessage::error(
            "The newsletter html content is empty.",
        )
        .send();
        return Ok(see_other("/admin/newsletters"));
    }

    if content_text.trim().is_empty() {
        FlashMessage::error(
            "The newsletter plain text content is empty.",
        )
        .send();
        return Ok(see_other("/admin/newsletters"));
    }
 
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        },
    };

    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                // No longer using `form.<X>`
                email_client
                    .send_email(&subscriber.email, &title, &content_html, &content_text)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500)?
            },
            Err(e) => tracing::warn!(
                    // We record the error chain as a structured field 
                    // on the log record.
                    error.cause_chain = ?e,
                    // Using `\` to split a long string literal over
                    // two lines, without creating a `\n` character.
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                ),
        }
    }

    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("Your newsletter has been published.")
}
