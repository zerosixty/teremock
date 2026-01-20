use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;

use super::{
    check_if_message_exists,
    common::{lock_state, RouteError, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::DeletedMessage, state::State};

#[derive(Debug, Deserialize, Clone)]
pub struct DeleteMessageBody {
    pub chat_id: BodyChatId,
    pub message_id: i32,
}

pub async fn delete_message(
    state: web::Data<Mutex<State>>,
    body: web::Json<DeleteMessageBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    check_if_message_exists!(lock, body.message_id, result);

    let deleted_message = lock
        .messages
        .delete_message(body.message_id)
        .ok_or_else(|| RouteError::bad_request("Message not found"))?;

    lock.responses.deleted_messages.push(DeletedMessage {
        message: deleted_message,
        bot_request: body.into_inner(),
    });

    Ok(make_telegram_result(true))
}
