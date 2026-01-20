use std::{collections::HashMap, sync::Mutex};

use actix_multipart::Multipart;
use actix_web::web;
use serde::Deserialize;
use teloxide::types::{BusinessConnectionId, EffectId, Me, ReplyMarkup, ReplyParameters, Seconds};

use super::{
    common::{generate_file_ids, lock_state, MessageSetup, RouteError, RouteResult},
    get_raw_multipart_fields, make_telegram_result, BodyChatId,
};
use crate::{
    proc_macros::SerializeRawFields,
    server::{
        routes::{Attachment, FileType, SerializeRawFields},
        SentMessageVideoNote,
    },
    state::State,
    MockMessageVideoNote,
};

pub async fn send_video_note(
    mut payload: Multipart,
    me: web::Data<Me>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let (fields, attachments) = get_raw_multipart_fields(&mut payload).await;
    let mut lock = lock_state(&state)?;

    let body =
        SendMessageVideoNoteBody::serialize_raw_fields(&fields, &attachments, FileType::Voice)
            .ok_or_else(|| RouteError::bad_request("Failed to parse request body"))?;

    let chat = body.chat_id.chat();
    let setup = MessageSetup::new(
        &me.user,
        body.protect_content,
        body.reply_parameters.as_ref(),
        body.reply_markup.as_ref(),
        &lock,
    )?;

    let mut message = MockMessageVideoNote::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;

    let (file_id, file_unique_id) = generate_file_ids();

    message.file_id = file_id;
    message.file_unique_id = file_unique_id;
    message.duration = body.duration.unwrap_or(Seconds::from_seconds(0));
    message.length = body.length.unwrap_or(100);
    message.file_size = body.file_data.len() as u32;
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.files.push(teloxide::types::File {
        meta: message.video_note().unwrap().file.clone(),
        path: body.file_name.to_owned(),
    });
    lock.responses.sent_messages.push(message.clone());
    lock.responses
        .sent_messages_video_note
        .push(SentMessageVideoNote {
            message: message.clone(),
            bot_request: body,
        });

    Ok(make_telegram_result(message))
}

#[derive(Debug, Clone, Deserialize, SerializeRawFields)]
pub struct SendMessageVideoNoteBody {
    pub chat_id: BodyChatId,
    pub message_thread_id: Option<i64>,
    pub file_name: String,
    pub file_data: String,
    pub duration: Option<Seconds>,
    pub length: Option<u32>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_parameters: Option<ReplyParameters>,
    pub reply_markup: Option<ReplyMarkup>,
    pub business_connection_id: Option<BusinessConnectionId>,
}
