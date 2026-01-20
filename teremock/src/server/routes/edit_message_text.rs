use std::sync::Mutex;

use actix_web::web;
use serde::Deserialize;
use teloxide::{
    types::{BusinessConnectionId, LinkPreviewOptions, MessageEntity, ParseMode, ReplyMarkup},
    ApiError,
};

use super::{
    common::{lock_state, RouteError, RouteResult},
    make_telegram_result, BodyChatId,
};
use crate::{server::EditedMessageText, state::State};

#[derive(Debug, Deserialize, Clone)]
pub struct EditMessageTextBody {
    pub chat_id: Option<BodyChatId>,
    pub message_id: Option<i32>,
    pub inline_message_id: Option<String>,
    pub text: String,
    pub parse_mode: Option<ParseMode>,
    pub entities: Option<Vec<MessageEntity>>,
    pub link_preview_options: Option<LinkPreviewOptions>,
    pub reply_markup: Option<ReplyMarkup>,
    pub business_connection_id: Option<BusinessConnectionId>,
}

pub async fn edit_message_text(
    body: web::Json<EditMessageTextBody>,
    state: web::Data<Mutex<State>>,
) -> RouteResult {
    match (
        body.chat_id.clone(),
        body.message_id,
        body.inline_message_id.clone(),
    ) {
        (Some(_), Some(message_id), None) => {
            let mut lock = lock_state(&state)?;
            let Some(old_message) = lock.messages.get_message(message_id) else {
                return Err(RouteError::from_api_error(ApiError::MessageToEditNotFound));
            };

            let old_reply_markup = old_message
                .reply_markup()
                .map(|kb| ReplyMarkup::InlineKeyboard(kb.clone()));
            if old_message.text() == Some(&body.text) && old_reply_markup == body.reply_markup {
                return Err(RouteError::from_api_error(ApiError::MessageNotModified));
            }

            lock.messages
                .edit_message_field(message_id, "text", body.text.clone());
            lock.messages.edit_message_field(
                message_id,
                "entities",
                body.entities.clone().unwrap_or_default(),
            );

            let message = lock
                .messages
                .edit_message_reply_markup(message_id, body.reply_markup.clone())
                .ok_or_else(|| RouteError::from_api_error(ApiError::MessageToEditNotFound))?;

            lock.responses.edited_messages_text.push(EditedMessageText {
                message: message.clone(),
                bot_request: body.into_inner(),
            });

            Ok(make_telegram_result(message))
        }
        // No implementation for inline messages yet, so just return success
        (None, None, Some(_)) => Ok(make_telegram_result(true)),
        _ => Err(RouteError::bad_request(
            "No message_id or inline_message_id were provided",
        )),
    }
}
