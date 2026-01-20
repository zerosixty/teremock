use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::{
    BusinessConnectionId, EffectId, LivePeriod, Me, ReplyMarkup, ReplyParameters,
};

use super::{
    common::{lock_state, MessageSetup, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::SentMessageLocation, state::State, MockMessageLocation};

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageLocationBody {
    pub chat_id: BodyChatId,
    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: Option<f64>,
    pub live_period: Option<LivePeriod>,
    pub heading: Option<u16>,
    pub proximity_alert_radius: Option<u32>,
    pub message_thread_id: Option<i64>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_location(
    body: web::Json<SendMessageLocationBody>,
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

    let mut message = MockMessageLocation::new()
        .chat(chat)
        .latitude(body.latitude)
        .longitude(body.longitude);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.horizontal_accuracy = body.horizontal_accuracy;
    message.live_period = body.live_period;
    message.heading = body.heading;
    message.proximity_alert_radius = body.proximity_alert_radius;
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.responses.sent_messages.push(message.clone());
    lock.responses
        .sent_messages_location
        .push(SentMessageLocation {
            message: message.clone(),
            bot_request: body.into_inner(),
        });

    Ok(make_telegram_result(message))
}
