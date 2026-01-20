use std::sync::Mutex;

use actix_web::web;
use chrono::DateTime;
use serde::Deserialize;
use teloxide::types::{
    BusinessConnectionId, EffectId, InputPollOption, Me, MessageEntity, ParseMode, PollOption,
    PollType, ReplyMarkup, ReplyParameters, Seconds,
};

use super::{
    common::{lock_state, MessageSetup, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::SentMessagePoll, state::State, MockMessagePoll};

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessagePollBody {
    pub chat_id: BodyChatId,
    pub message_thread_id: Option<i64>,
    pub question: String,
    pub question_parse_mode: Option<ParseMode>,
    pub question_entities: Option<Vec<MessageEntity>>,
    pub options: Vec<InputPollOption>,
    pub is_anonymous: Option<bool>,
    pub r#type: Option<PollType>,
    pub allows_multiple_answers: Option<bool>,
    pub correct_option_id: Option<u8>,
    pub explanation: Option<String>,
    pub explanation_parse_mode: Option<ParseMode>,
    pub explanation_entities: Option<Vec<MessageEntity>>,
    pub open_period: Option<Seconds>,
    pub close_date: Option<u16>,
    pub is_closed: Option<bool>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_poll(
    state: web::Data<Mutex<State>>,
    body: web::Json<SendMessagePollBody>,
    me: web::Data<Me>,
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

    let mut message = MockMessagePoll::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.business_connection_id = body.business_connection_id.clone();

    message.question = body.question.clone();
    let options: Vec<PollOption> = body
        .options
        .iter()
        .map(|option| PollOption {
            text: option.text.clone(),
            text_entities: None,
            voter_count: 0,
        })
        .collect();
    message.options = options;
    message.is_anonymous = body.is_anonymous.unwrap_or(false);
    message.poll_type = body.r#type.clone().unwrap_or(PollType::Regular);
    message.allows_multiple_answers = body.allows_multiple_answers.unwrap_or(false);
    message.correct_option_id = body.correct_option_id;
    message.explanation = body.explanation.clone();
    message.explanation_entities = body.explanation_entities.clone();
    message.open_period = body.open_period;
    message.close_date = DateTime::from_timestamp(body.close_date.unwrap_or(0) as i64, 0);
    message.effect_id = body.message_effect_id.clone();
    message.question_entities = body.question_entities.clone();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.responses.sent_messages.push(message.clone());
    lock.responses.sent_messages_poll.push(SentMessagePoll {
        message: message.clone(),
        bot_request: body.into_inner(),
    });

    Ok(make_telegram_result(message))
}
