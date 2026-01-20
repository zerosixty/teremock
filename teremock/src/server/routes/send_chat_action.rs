use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::BusinessConnectionId;

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct SendChatActionBody {
    pub chat_id: BodyChatId,
    pub message_thread_id: Option<i64>,
    pub action: String,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn send_chat_action(
    state: web::Data<Mutex<State>>,
    body: web::Json<SendChatActionBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    lock.responses.sent_chat_actions.push(body.into_inner());
    Ok(make_telegram_result(true))
}
