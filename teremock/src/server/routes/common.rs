//! Common helper functions for route handlers to reduce code duplication.

use std::sync::{Mutex, MutexGuard, PoisonError};

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use rand::distr::{Alphanumeric, SampleString};
use teloxide::types::{
    FileId, FileUniqueId, InlineKeyboardMarkup, Message, ReplyMarkup, ReplyParameters, User,
};

use crate::state::State;

/// Default chat ID used when a text username is provided instead of a numeric ID.
/// This is a placeholder value for username-based chat lookups which aren't fully supported.
pub const DEFAULT_TEXT_CHAT_ID: i64 = 123456789;

/// Length of generated file IDs (matches Telegram's typical length).
pub const FILE_ID_LENGTH: usize = 16;

/// Length of generated file unique IDs.
pub const FILE_UNIQUE_ID_LENGTH: usize = 8;

/// Default dimension (width/height) for media files when not specified.
pub const DEFAULT_MEDIA_DIMENSION: u32 = 100;

/// Default duration in seconds for media files when not specified.
pub const DEFAULT_MEDIA_DURATION_SECS: u32 = 1;

/// Default MIME type for video files.
pub const DEFAULT_VIDEO_MIME_TYPE: &str = "video/mp4";

/// Default MIME type for audio files.
pub const DEFAULT_AUDIO_MIME_TYPE: &str = "audio/mp3";

/// Error type for route handlers.
///
/// This allows us to use `Result<HttpResponse, RouteError>` as a return type,
/// which properly implements `Responder` through actix-web's error handling.
#[derive(Debug, Clone)]
pub struct RouteError {
    status: StatusCode,
    body: String,
}

impl RouteError {
    pub fn new(response: HttpResponse) -> Self {
        // We can't easily extract the body from HttpResponse, so this constructor
        // is primarily for compatibility. Use the specific constructors when possible.
        Self {
            status: response.status(),
            body: r#"{"ok":false,"description":"Unknown error"}"#.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_status_and_body(status: StatusCode, body: String) -> Self {
        Self { status, body }
    }

    pub fn bad_request(message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: format!(r#"{{"ok":false,"description":"{}"}}"#, message),
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: format!(r#"{{"ok":false,"description":"{}"}}"#, message),
        }
    }

    /// Creates a RouteError from a teloxide ApiError.
    ///
    /// This preserves the error message for proper error responses.
    pub fn from_api_error(error: teloxide::ApiError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: format!(r#"{{"ok":false,"description":"{}"}}"#, error),
        }
    }
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Route error: {}", self.body)
    }
}

impl ResponseError for RouteError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status)
            .content_type("application/json")
            .body(self.body.clone())
    }
}

/// Result type alias for route handlers.
pub type RouteResult = Result<HttpResponse, RouteError>;

/// Attempts to lock the state mutex, returning an appropriate error on failure.
///
/// This replaces the `.lock().unwrap()` pattern with proper error handling.
pub fn lock_state(state: &Mutex<State>) -> Result<MutexGuard<'_, State>, RouteError> {
    state.lock().map_err(|_: PoisonError<_>| {
        log::error!("State mutex was poisoned");
        RouteError::internal_error("Internal server error: state lock poisoned")
    })
}

/// Generates a random file ID with the standard length.
pub fn generate_file_id() -> FileId {
    FileId(Alphanumeric.sample_string(&mut rand::rng(), FILE_ID_LENGTH))
}

/// Generates a random file unique ID with the standard length.
pub fn generate_file_unique_id() -> FileUniqueId {
    FileUniqueId(Alphanumeric.sample_string(&mut rand::rng(), FILE_UNIQUE_ID_LENGTH))
}

/// Generates both file ID and file unique ID as a tuple.
pub fn generate_file_ids() -> (FileId, FileUniqueId) {
    (generate_file_id(), generate_file_unique_id())
}

/// Helper to extract inline keyboard from reply markup, if present.
pub fn extract_inline_keyboard(markup: Option<&ReplyMarkup>) -> Option<InlineKeyboardMarkup> {
    match markup {
        Some(ReplyMarkup::InlineKeyboard(kb)) => Some(kb.clone()),
        _ => None,
    }
}

/// Sets common message fields from a request body.
///
/// This function handles the common pattern of:
/// - Setting `from` to the bot user
/// - Setting `has_protected_content`
/// - Looking up and setting `reply_to_message` if reply_parameters provided
/// - Setting `reply_markup` if it's an inline keyboard
///
/// Returns the reply_to_message if found, or an error if the referenced message doesn't exist.
pub fn setup_reply_to_message(
    lock: &MutexGuard<'_, State>,
    reply_parameters: Option<&ReplyParameters>,
) -> Result<Option<Box<Message>>, RouteError> {
    if let Some(params) = reply_parameters {
        let message_id = params.message_id.0;
        match lock.messages.get_message(message_id) {
            Some(msg) => Ok(Some(Box::new(msg))),
            None => Err(RouteError::bad_request(
                "Bad Request: message to reply not found",
            )),
        }
    } else {
        Ok(None)
    }
}

/// Registers a file in the state for later retrieval via GetFile.
#[allow(dead_code)]
pub fn register_file(
    lock: &mut MutexGuard<'_, State>,
    file_meta: teloxide::types::FileMeta,
    path: String,
) {
    lock.files.push(teloxide::types::File {
        meta: file_meta,
        path,
    });
}

/// Common setup for media messages: sets from, protected content, and handles reply.
///
/// This is a helper struct to collect common message setup operations.
pub struct MessageSetup {
    pub from: Option<User>,
    pub has_protected_content: bool,
    pub reply_to_message: Option<Box<Message>>,
    pub reply_markup: Option<InlineKeyboardMarkup>,
}

impl MessageSetup {
    /// Creates a new MessageSetup from common request fields.
    ///
    /// Returns an error if reply_parameters references a non-existent message.
    pub fn new(
        me_user: &User,
        protect_content: Option<bool>,
        reply_parameters: Option<&ReplyParameters>,
        reply_markup: Option<&ReplyMarkup>,
        state_lock: &MutexGuard<'_, State>,
    ) -> Result<Self, RouteError> {
        let reply_to_message = setup_reply_to_message(state_lock, reply_parameters)?;

        Ok(Self {
            from: Some(me_user.clone()),
            has_protected_content: protect_content.unwrap_or(false),
            reply_to_message,
            reply_markup: extract_inline_keyboard(reply_markup),
        })
    }
}
