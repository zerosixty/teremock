use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct BanChatMemberBody {
    pub chat_id: BodyChatId,
    pub user_id: u64,
    pub until_date: Option<i64>,
    pub revoke_messages: Option<bool>,
}

pub async fn ban_chat_member(
    state: web::Data<Mutex<State>>,
    body: web::Json<BanChatMemberBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    let chat_id = body.chat_id.id();
    if body.revoke_messages.unwrap_or(false) {
        let to_delete: Vec<_> = lock
            .messages
            .messages
            .iter()
            .filter(|m| {
                m.chat.id.0 == chat_id && m.from.as_ref().map(|u| u.id.0) == Some(body.user_id)
            })
            .map(|m| m.id.0)
            .collect();
        for id in to_delete {
            lock.messages.delete_message(id);
        }
    }
    lock.responses.banned_chat_members.push(body.into_inner());

    Ok(make_telegram_result(true))
}
