use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::{BusinessConnectionId, EffectId, Me, ReplyMarkup, ReplyParameters};

use super::{
    common::{lock_state, MessageSetup, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::SentMessageVenue, state::State, MockLocation, MockMessageVenue};

#[derive(Debug, Deserialize, Clone)]
pub struct SendMessageVenueBody {
    pub chat_id: BodyChatId,
    pub message_thread_id: Option<i64>,
    pub latitude: f64,
    pub longitude: f64,
    pub title: String,
    pub address: String,
    pub foursquare_id: Option<String>,
    pub foursquare_type: Option<String>,
    pub google_place_id: Option<String>,
    pub google_place_type: Option<String>,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub message_effect_id: Option<EffectId>,
    pub reply_markup: Option<ReplyMarkup>,
    pub reply_parameters: Option<ReplyParameters>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_venue(
    body: web::Json<SendMessageVenueBody>,
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

    let mut message = MockMessageVenue::new().chat(chat);
    message.from = setup.from;
    message.has_protected_content = setup.has_protected_content;
    message.reply_to_message = setup.reply_to_message;
    message.reply_markup = setup.reply_markup;
    message.location = MockLocation::new()
        .latitude(body.latitude)
        .longitude(body.longitude)
        .build();
    message.title = body.title.clone();
    message.address = body.address.clone();
    message.foursquare_id = body.foursquare_id.clone();
    message.foursquare_type = body.foursquare_type.clone();
    message.google_place_id = body.google_place_id.clone();
    message.google_place_type = body.google_place_type.clone();
    message.effect_id = body.message_effect_id.clone();
    message.business_connection_id = body.business_connection_id.clone();

    let last_id = lock.messages.max_message_id();
    let message = lock.messages.add_message(message.id(last_id + 1).build());

    lock.responses.sent_messages.push(message.clone());
    lock.responses.sent_messages_venue.push(SentMessageVenue {
        message: message.clone(),
        bot_request: body.into_inner(),
    });

    Ok(make_telegram_result(message))
}
