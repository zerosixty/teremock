use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;

use super::{
    common::{lock_state, RouteResult},
    make_telegram_result,
};
use crate::state::State;

#[derive(Debug, Deserialize, Clone)]
pub struct AnswerCallbackQueryBody {
    pub callback_query_id: String,
    pub text: Option<String>,
    pub show_alert: Option<bool>,
    pub url: Option<String>,
    pub cache_time: Option<i32>,
}

pub async fn answer_callback_query(
    state: web::Data<Mutex<State>>,
    body: web::Json<AnswerCallbackQueryBody>,
) -> RouteResult {
    let mut lock = lock_state(&state)?;
    lock.responses
        .answered_callback_queries
        .push(body.into_inner());
    Ok(make_telegram_result(true))
}
