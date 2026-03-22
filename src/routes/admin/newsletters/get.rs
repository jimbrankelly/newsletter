use actix_web::{
    HttpResponse,
    http::header::ContentType,
    web,
};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::authentication::UserId;

pub async fn publish_newsletter_form(
    _user_id: web::ReqData<UserId>,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {

    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Issue a newsletter</title>
</head>
<body>
    {msg_html}
    <form action="/admin/newsletters" method="post">
        <label>Title
            <input
                type="text"
                placeholder="Enter newsletter title"
                name="title"
            >
        </label>
        <br>
        <label>Html Content
            <input
                type="text"
                placeholder="Enter newsletter html content"
                name="content_html"
            >
        </label>
        <label>Text Content
            <input
                type="text"
                placeholder="Enter newsletter text content"
                name="content_text"
            >
        </label>
        <br>
        <button type="submit">Publish newsletter</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
    )))
}
