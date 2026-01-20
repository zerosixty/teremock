use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct UnpinAllChatMessagesBody {
    pub chat_id: BodyChatId,
}

pub async fn unpin_all_chat_messages(
    state: web::Data<Mutex<State>>,
    body: web::Json<UnpinAllChatMessagesBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    lock.responses
        .unpinned_all_chat_messages
        .push(body.into_inner());

    Ok(make_telegram_result(true))
}
