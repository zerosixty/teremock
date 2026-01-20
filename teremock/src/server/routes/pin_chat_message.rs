use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::BusinessConnectionId;

use super::{
    check_if_message_exists,
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct PinChatMessageBody {
    pub chat_id: BodyChatId,
    pub message_id: i32,
    pub disable_notification: Option<bool>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn pin_chat_message(
    state: web::Data<Mutex<State>>,
    body: web::Json<PinChatMessageBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    check_if_message_exists!(lock, body.message_id, result);
    lock.responses.pinned_chat_messages.push(body.into_inner());
    Ok(make_telegram_result(true))
}
