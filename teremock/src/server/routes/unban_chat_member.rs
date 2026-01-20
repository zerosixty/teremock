use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct UnbanChatMemberBody {
    pub chat_id: BodyChatId,
    pub user_id: u64,
    pub only_if_banned: Option<bool>,
}

pub async fn unban_chat_member(
    state: web::Data<Mutex<State>>,
    body: web::Json<UnbanChatMemberBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    lock.responses.unbanned_chat_members.push(body.into_inner());
    Ok(make_telegram_result(true))
}
