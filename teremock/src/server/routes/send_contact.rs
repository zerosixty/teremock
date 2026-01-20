use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::{BusinessConnectionId, EffectId, Me, ReplyMarkup, ReplyParameters};

use super::{
    common::{lock_state, MessageSetup, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::SentMessageContact, state::State, MockMessageContact};

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageContactBody {
    pub chat_id: BodyChatId,
    pub message_thread_id: Option<i64>,
    pub phone_number: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub vcard: Option<String>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_contact(
    body: web::Json<SendMessageContactBody>,
    me: web::Data<Me>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;

    let chat = body.chat_id.chat();
    let setup = MessageSetup::new(
        &me.user,
        body.protect_content,
        body.reply_parameters.as_ref(),
        body.reply_markup.as_ref(),
        &lock,
    )?;

    let mut message = MockMessageContact::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.phone_number = body.phone_number.clone();
    message.first_name = body.first_name.clone();
    message.last_name = body.last_name.clone();
    message.vcard = body.vcard.clone();
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.responses.sent_messages.push(message.clone());
    lock.responses
        .sent_messages_contact
        .push(SentMessageContact {
            message: message.clone(),
            bot_request: body.into_inner(),
        });

    Ok(make_telegram_result(message))
}
