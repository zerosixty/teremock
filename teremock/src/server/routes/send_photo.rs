use std::{collections::HashMap, sync::Mutex};

use actix_multipart::Multipart;
use actix_web::web;
use serde::Deserialize;
use teloxide::types::{
    BusinessConnectionId, EffectId, LinkPreviewOptions, Me, MessageEntity, ParseMode, ReplyMarkup,
    ReplyParameters,
};

use super::{
    common::{generate_file_ids, lock_state, MessageSetup, RouteError, RouteResult},
    get_raw_multipart_fields, make_telegram_result, BodyChatId,
};
use crate::{
    dataset::{MockMessagePhoto, MockPhotoSize},
    proc_macros::SerializeRawFields,
    server::{
        routes::{Attachment, FileType, SerializeRawFields},
        SentMessagePhoto,
    },
    state::State,
};

pub async fn send_photo(
    mut payload: Multipart,
    me: web::Data<Me>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let (fields, attachments) = get_raw_multipart_fields(&mut payload).await;
    let mut lock = lock_state(&state)?;

    let body = SendMessagePhotoBody::serialize_raw_fields(&fields, &attachments, FileType::Photo)
        .ok_or_else(|| RouteError::bad_request("Failed to parse request body"))?;

    let chat = body.chat_id.chat();
    let setup = MessageSetup::new(
        &me.user,
        body.protect_content,
        body.reply_parameters.as_ref(),
        body.reply_markup.as_ref(),
        &lock,
    )?;

    let mut message = MockMessagePhoto::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.caption = body.caption.clone();
    message.caption_entities = body.caption_entities.clone().unwrap_or_default();
    message.show_caption_above_media = body.show_caption_above_media.unwrap_or(false);
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let (file_id, file_unique_id) = generate_file_ids();

    message.photo = vec![MockPhotoSize::new()
        .file_id(file_id)
        .file_unique_id(file_unique_id)
        .file_size(body.file_data.len() as u32)
        .build()];

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    if let Some(photo) = message.photo() {
        if let Some(first_photo) = photo.first() {
            lock.files.push(teloxide::types::File {
                meta: first_photo.file.clone(),
                path: body.file_name.clone(),
            });
        }
    }

    lock.responses.sent_messages.push(message.clone());
    lock.responses.sent_messages_photo.push(SentMessagePhoto {
        message: message.clone(),
        bot_request: body,
    });

    Ok(make_telegram_result(message))
}

#[derive(Debug, Clone, Deserialize, SerializeRawFields)]
pub struct SendMessagePhotoBody {
    pub chat_id: BodyChatId,
    pub file_name: String,
    pub file_data: String,
    pub caption: Option<String>,
    pub message_thread_id: Option<i64>,
    pub parse_mode: Option<ParseMode>,
    pub caption_entities: Option<Vec<MessageEntity>>,
    pub link_preview_options: Option<LinkPreviewOptions>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub show_caption_above_media: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}
