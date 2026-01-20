//! # teremock - Production-grade Mock Bot for Teloxide Integration Testing
//!
//! A high-performance mock bot for integration testing teloxide bots with an actual fake server.
//!
//! ## Key Features
//!
//! - **Persistent Server Architecture**: Server starts once and is reused across all dispatches
//! - **Stack Overflow Prevention**: Uses tokio task spawn per dispatch to prevent stack buildup
//! - **Black-Box Testing**: No dialogue state manipulation - tests interact only through the bot interface
//! - **Rich Response Inspection**: Comprehensive access to all bot API responses
//!
//! ## Quick Start
//!
//! ```no_run
//! use teloxide::{
//!     dispatching::{UpdateFilterExt, UpdateHandler},
//!     prelude::*,
//! };
//!
//! type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
//!
//! async fn hello_world(bot: Bot, message: Message) -> HandlerResult {
//!     bot.send_message(message.chat.id, "Hello World!").await?;
//!     Ok(())
//! }
//!
//! fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
//!     dptree::entry().branch(Update::filter_message().endpoint(hello_world))
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!     use teremock::{MockBot, MockMessageText};
//!
//!     #[tokio::test]
//!     async fn test_hello_world() {
//!         let mut bot = MockBot::new(MockMessageText::new().text("Hi!"), handler_tree()).await;
//!         bot.dispatch().await;
//!         let message = bot.get_responses().sent_messages.last().unwrap();
//!         assert_eq!(message.text(), Some("Hello World!"));
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! teremock is designed for production-grade integration testing with:
//!
//! 1. **Persistent Server**: Unlike the original teloxide_tests where each dispatch creates a new
//!    server, teremock keeps the server alive across all dispatches. This provides 15-30x faster
//!    test execution (2s vs 30-60s for 50+ dispatches).
//!
//! 2. **Tokio Task Isolation**: Each dispatch runs in a separate tokio task with a fresh 2MB stack.
//!    This prevents stack overflow issues that occur when handler trees are cloned across many
//!    sequential dispatches.
//!
//! 3. **Black-Box Testing Philosophy**: Tests should interact only through the bot interface
//!    (messages, callbacks, commands). There's no dialogue state manipulation API - state changes
//!    happen naturally through the handler tree.
//!
//! ## Supported Endpoints
//!
//! - /AnswerCallbackQuery
//! - /DeleteMessage
//! - /DeleteMessages
//! - /EditMessageText
//! - /EditMessageReplyMarkup
//! - /EditMessageCaption
//! - /GetFile
//! - /SendMessage
//! - /SendDocument
//! - /SendPhoto
//! - /SendVideo
//! - /SendAudio
//! - /SendVoice
//! - /SendVideoNote
//! - /SendAnimation
//! - /SendLocation
//! - /SendVenue
//! - /SendContact
//! - /SendDice
//! - /SendPoll
//! - /SendSticker
//! - /SendChatAction
//! - /SendMediaGroup
//! - /SendInvoice
//! - /PinChatMessage
//! - /UnpinChatMessage
//! - /UnpinAllChatMessages
//! - /ForwardMessage
//! - /CopyMessage
//! - /BanChatMember
//! - /UnbanChatMember
//! - /RestrictChatMember
//! - /SetMessageReaction
//! - /SetMyCommands
//! - /GetMe
//!
//! ## Migration from teloxide_tests
//!
//! The main API differences:
//!
//! ```ignore
//! // OLD (teloxide_tests):
//! let mut bot = MockBot::new(MockMessageText::new().text("Hi!"), handler_tree());
//! bot.dispatch().await;
//!
//! // NEW (teremock):
//! let mut bot = MockBot::new(MockMessageText::new().text("Hi!"), handler_tree()).await;
//! bot.dispatch().await;
//! // Note: `new()` is now async because it starts the server immediately
//! ```
//!
//! Key differences:
//! - `new()` is now async (starts the server immediately)
//! - No `set_state()` / `get_state()` methods (black-box testing)
//! - Server persists across dispatches (much faster)
//! - Works with default 2MB stack (no custom thread builder needed)
//! - No global lock needed (server is persistent per MockBot instance)
#![doc(
    html_logo_url = "https://github.com/user-attachments/assets/627beca8-5852-4c70-97e0-5f4fcb5e2040",
    html_favicon_url = "https://github.com/user-attachments/assets/627beca8-5852-4c70-97e0-5f4fcb5e2040"
)]
// Clippy suppressions - these are intentional design choices:
// - new_without_default: 36 Mock* builders have new() but Default adds no value for builder pattern
// - too_many_arguments: Macro-generated constructors for Telegram API types
// - enum_variant_names: InputMedia* variants mirror Telegram API naming
#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::enum_variant_names)]

mod dataset;
mod mock_bot;
pub mod server;
pub(crate) mod state;
#[cfg(test)]
mod tests;
pub(crate) mod utils;

pub use dataset::*;
pub use mock_bot::{DistributionKey, MockBot};
pub use server::Responses;
use teloxide::types::{ChatId, MessageId, UserId};
use teremock_macros as proc_macros;

/// Error type alias commonly used with handler trees
pub type HandlerError = Box<dyn std::error::Error + Send + Sync + 'static>;

// Conversion traits for ergonomic ID field setters
// These traits allow mock builders to accept both primitive types and teloxide wrapper types

/// Trait for types that can be converted to [`ChatId`].
///
/// This trait is automatically implemented for:
/// - `i64` - raw chat ID value
/// - `i32` - raw chat ID value (for bare integer literals)
/// - `ChatId` - directly pass a ChatId
/// - `UserId` - for private chats where chat ID equals user ID
pub trait IntoChatId {
    fn into_chat_id(self) -> ChatId;
}

impl IntoChatId for i64 {
    fn into_chat_id(self) -> ChatId {
        ChatId(self)
    }
}

impl IntoChatId for i32 {
    fn into_chat_id(self) -> ChatId {
        ChatId(self as i64)
    }
}

impl IntoChatId for ChatId {
    fn into_chat_id(self) -> ChatId {
        self
    }
}

impl IntoChatId for UserId {
    fn into_chat_id(self) -> ChatId {
        ChatId(self.0 as i64)
    }
}

/// Trait for types that can be converted to [`UserId`].
///
/// This trait is automatically implemented for:
/// - `u64` - raw user ID value
/// - `i64` - raw user ID value (useful when working with chat IDs)
/// - `i32` - raw user ID value (for bare integer literals)
/// - `UserId` - directly pass a UserId
pub trait IntoUserId {
    fn into_user_id(self) -> UserId;
}

impl IntoUserId for u64 {
    fn into_user_id(self) -> UserId {
        UserId(self)
    }
}

impl IntoUserId for i64 {
    fn into_user_id(self) -> UserId {
        UserId(self as u64)
    }
}

impl IntoUserId for i32 {
    fn into_user_id(self) -> UserId {
        UserId(self as u64)
    }
}

impl IntoUserId for UserId {
    fn into_user_id(self) -> UserId {
        self
    }
}

/// Trait for types that can be converted to [`MessageId`].
///
/// This trait is automatically implemented for:
/// - `i32` - raw message ID value
/// - `MessageId` - directly pass a MessageId
pub trait IntoMessageId {
    fn into_message_id(self) -> MessageId;
}

impl IntoMessageId for i32 {
    fn into_message_id(self) -> MessageId {
        MessageId(self)
    }
}

impl IntoMessageId for MessageId {
    fn into_message_id(self) -> MessageId {
        self
    }
}
