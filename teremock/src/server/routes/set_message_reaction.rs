use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::ReactionType;

use super::{
    check_if_message_exists,
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct SetMessageReactionBody {
    pub chat_id: BodyChatId,
    pub message_id: i32,
    pub reaction: Option<Vec<ReactionType>>,
    pub is_big: Option<bool>,
}

pub async fn set_message_reaction(
    state: web::Data<Mutex<State>>,
    body: web::Json<SetMessageReactionBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    check_if_message_exists!(lock, body.message_id, result);
    lock.responses.set_message_reaction.push(body.into_inner());
    Ok(make_telegram_result(true))
}
