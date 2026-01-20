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
pub struct UnpinChatMessageBody {
    pub chat_id: BodyChatId,
    pub message_id: Option<i32>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn unpin_chat_message(
    state: web::Data<Mutex<State>>,
    body: web::Json<UnpinChatMessageBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    if let Some(message_id) = body.message_id {
        check_if_message_exists!(lock, message_id, result);
    }
    lock.responses
        .unpinned_chat_messages
        .push(body.into_inner());

    Ok(make_telegram_result(true))
}
