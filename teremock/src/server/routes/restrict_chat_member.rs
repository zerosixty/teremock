use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::ChatPermissions;

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct RestrictChatMemberBody {
    pub chat_id: BodyChatId,
    pub user_id: u64,
    pub permissions: ChatPermissions,
    pub use_independent_chat_permissions: Option<bool>,
    pub until_date: Option<i64>,
}

pub async fn restrict_chat_member(
    state: web::Data<Mutex<State>>,
    body: web::Json<RestrictChatMemberBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    lock.responses
        .restricted_chat_members
        .push(body.into_inner());
    Ok(make_telegram_result(true))
}
