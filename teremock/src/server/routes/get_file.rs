use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::types::FileId;

use super::common::{lock_state, RouteError, RouteResult};
use super::make_telegram_result;
use crate::state::State;

#[derive(Deserialize)]
pub struct GetFileQuery {
    file_id: FileId,
}

pub async fn get_file(
    query: web::Json<GetFileQuery>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    let lock = lock_state(&state)?;
    let Some(file) = lock.files.iter().find(|f| f.id == query.file_id) else {
        return Err(RouteError::bad_request("File not found"));
    };
    Ok(make_telegram_result(file))
}
