use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::{BusinessConnectionId, DiceEmoji, ReplyMarkup, ReplyParameters};

use super::{
    common::{lock_state, setup_reply_to_message, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::SentMessageDice, state::State, MockMessageDice};

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageDiceBody {
    pub chat_id: BodyChatId,
    pub message_thread_id: Option<i64>,
    pub emoji: Option<DiceEmoji>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<String>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_dice(
    state: web::Data<Mutex<State>>,
    body: web::Json<SendMessageDiceBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;

    let chat = body.chat_id.chat();

    // Validate reply_parameters if provided (returns error if message doesn't exist)
    let _reply_to = setup_reply_to_message(&lock, body.reply_parameters.as_ref())?;

    let mut message = MockMessageDice::new().chat(chat);
    message.emoji = body.emoji.unwrap_or(MockMessageDice::EMOJI);
    // Random from 1 to 5 because it fits all the emoji
    message.value = (1 + rand::random::<u8>() % 5) as u8;

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.responses.sent_messages.push(message.clone());
    lock.responses.sent_messages_dice.push(SentMessageDice {
        message: message.clone(),
        bot_request: body.into_inner(),
    });

    Ok(make_telegram_result(message))
}
