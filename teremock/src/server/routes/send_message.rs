use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::{
    BusinessConnectionId, EffectId, LinkPreviewOptions, Me, MessageEntity, ParseMode, ReplyMarkup,
    ReplyParameters,
};

use super::{
    common::{lock_state, MessageSetup, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{dataset::message_common::MockMessageText, server::SentMessageText, state::State};

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageTextBody {
    pub chat_id: BodyChatId,
    pub text: String,
    pub message_thread_id: Option<i64>,
    pub parse_mode: Option<ParseMode>,
    pub entities: Option<Vec<MessageEntity>>,
    pub link_preview_options: Option<LinkPreviewOptions>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_message(
    body: web::Json<SendMessageTextBody>,
    me: web::Data<Me>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    let body = body.into_inner();
    let chat = body.chat_id.chat();

    let setup = MessageSetup::new(
        &me.user,
        body.protect_content,
        body.reply_parameters.as_ref(),
        body.reply_markup.as_ref(),
        &lock,
    )?;

    let mut message = MockMessageText::new().text(&body.text).chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();
    message.entities = body.entities.clone().unwrap_or_default();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.responses.sent_messages.push(message.clone());
    lock.responses.sent_messages_text.push(SentMessageText {
        message: message.clone(),
        bot_request: body,
    });

    Ok(make_telegram_result(message))
}
