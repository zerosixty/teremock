use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::{BotCommand, BotCommandScope};

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct SetMyCommandsBody {
    pub commands: Vec<BotCommand>,
    pub scope: Option<BotCommandScope>,
    pub language_code: Option<String>,
}

pub async fn set_my_commands(
    state: web::Data<Mutex<State>>,
    body: web::Json<SetMyCommandsBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    lock.responses.set_my_commands.push(body.into_inner());
    Ok(make_telegram_result(true))
}
