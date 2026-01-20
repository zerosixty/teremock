use std::{collections::HashMap, sync::Mutex};

use actix_multipart::Multipart;
use actix_web::web;
use serde::Deserialize;
use teloxide::types::{BusinessConnectionId, EffectId, Me, ReplyMarkup, ReplyParameters};

use super::{
    common::{lock_state, MessageSetup, RouteError, RouteResult},
    get_raw_multipart_fields, make_telegram_result, BodyChatId,
};
use crate::{
    proc_macros::SerializeRawFields,
    server::{
        routes::{Attachment, FileType, SerializeRawFields},
        SentMessageSticker,
    },
    state::State,
    MockMessageSticker,
};

pub async fn send_sticker(
    mut payload: Multipart,
    me: web::Data<Me>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let (fields, attachments) = get_raw_multipart_fields(&mut payload).await;
    let mut lock = lock_state(&state)?;

    let body =
        SendMessageStickerBody::serialize_raw_fields(&fields, &attachments, FileType::Sticker)
            .ok_or_else(|| RouteError::bad_request("Failed to parse request body"))?;

    let chat = body.chat_id.chat();
    let setup = MessageSetup::new(
        &me.user,
        body.protect_content,
        body.reply_parameters.as_ref(),
        body.reply_markup.as_ref(),
        &lock,
    )?;

    let mut message = MockMessageSticker::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.emoji = body.emoji.clone();
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.files.push(teloxide::types::File {
        meta: message.sticker().unwrap().file.clone(),
        path: body.file_name.to_owned(),
    });
    lock.responses.sent_messages.push(message.clone());
    lock.responses
        .sent_messages_sticker
        .push(SentMessageSticker {
            message: message.clone(),
            bot_request: body,
        });

    Ok(make_telegram_result(message))
}

#[derive(Debug, Clone, Deserialize, SerializeRawFields)]
pub struct SendMessageStickerBody {
    pub chat_id: BodyChatId,
    pub file_name: String,
    pub file_data: String,
    pub message_thread_id: Option<i64>,
    pub emoji: Option<String>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}
