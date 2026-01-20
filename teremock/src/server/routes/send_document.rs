use std::{collections::HashMap, str::FromStr, sync::Mutex};

use actix_multipart::Multipart;
use actix_web::web;
use mime::Mime;
use serde::Deserialize;
use teloxide::types::{
    BusinessConnectionId, EffectId, Me, MessageEntity, ParseMode, ReplyMarkup, ReplyParameters,
};

use super::{
    common::{generate_file_ids, lock_state, MessageSetup, RouteError, RouteResult},
    get_raw_multipart_fields, make_telegram_result, BodyChatId,
};
use crate::{
    dataset::MockMessageDocument,
    proc_macros::SerializeRawFields,
    server::{
        routes::{Attachment, FileType, SerializeRawFields},
        SentMessageDocument,
    },
    state::State,
};

pub async fn send_document(
    mut payload: Multipart,
    me: web::Data<Me>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let (fields, attachments) = get_raw_multipart_fields(&mut payload).await;
    let mut lock = lock_state(&state)?;

    let body =
        SendMessageDocumentBody::serialize_raw_fields(&fields, &attachments, FileType::Document)
            .ok_or_else(|| RouteError::bad_request("Failed to parse request body"))?;

    let chat = body.chat_id.chat();
    let setup = MessageSetup::new(
        &me.user,
        body.protect_content,
        body.reply_parameters.as_ref(),
        body.reply_markup.as_ref(),
        &lock,
    )?;

    let mut message = MockMessageDocument::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.caption = body.caption.clone();
    message.caption_entities = body.caption_entities.clone().unwrap_or_default();
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let (file_id, file_unique_id) = generate_file_ids();

    message.file_name = Some(body.file_name.clone());
    message.file_id = file_id;
    message.file_unique_id = file_unique_id;
    message.file_size = body.file_data.len() as u32;
    message.mime_type = Some(
        mime_guess::from_path(body.file_name.clone())
            .first()
            .unwrap_or(Mime::from_str("text/plain").unwrap()),
    );

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.files.push(teloxide::types::File {
        meta: message.document().unwrap().file.clone(),
        path: body.file_name.to_owned(),
    });
    lock.responses.sent_messages.push(message.clone());
    lock.responses
        .sent_messages_document
        .push(SentMessageDocument {
            message: message.clone(),
            bot_request: body,
        });

    Ok(make_telegram_result(message))
}

#[derive(Debug, Clone, Deserialize, SerializeRawFields)]
pub struct SendMessageDocumentBody {
    pub chat_id: BodyChatId,
    pub file_name: String,
    pub file_data: String,
    pub caption: Option<String>,
    pub message_thread_id: Option<i64>,
    pub parse_mode: Option<ParseMode>,
    pub caption_entities: Option<Vec<MessageEntity>>,
    pub disable_content_type_detection: Option<bool>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}
