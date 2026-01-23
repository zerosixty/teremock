# teremock

**Te**legram · **Re**alistic · **Mock**ing — A fast, ergonomic testing library for [teloxide](https://github.com/teloxide/teloxide) bots.

[![Crates.io](https://img.shields.io/crates/v/teremock.svg)](https://crates.io/crates/teremock)
[![Downloads](https://img.shields.io/crates/d/teremock.svg)](https://crates.io/crates/teremock)
[![Documentation](https://docs.rs/teremock/badge.svg)](https://docs.rs/teremock)
[![CI](https://github.com/zerosixty/teremock/actions/workflows/ci.yml/badge.svg)](https://github.com/zerosixty/teremock/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.83-blue)](https://blog.rust-lang.org/2024/11/28/Rust-1.83.0.html)
[![teloxide](https://img.shields.io/badge/teloxide-0.17-green)](https://github.com/teloxide/teloxide)
[![License](https://img.shields.io/crates/l/teremock.svg)](LICENSE)

---

teremock enables you to write fast, reliable integration tests for your Telegram bots without network access, API tokens, or external services. Run your entire test suite in seconds, not minutes.

```rust
use teremock::{MockBot, MockMessageText};

#[tokio::test]
async fn test_hello_command() {
    let mut bot = MockBot::new(MockMessageText::new().text("/hello"), handler_tree()).await;

    bot.dispatch().await;

    let responses = bot.get_responses();
    assert_eq!(responses.sent_messages.last().unwrap().text(), Some("Hello, World!"));
}
```

## Why teremock?

### Lightning Fast

Tests run **15-30x faster** than traditional approaches. The mock server starts once and persists across all dispatches within a test — no server restart overhead between interactions.

```
50 sequential dispatches: ~2 seconds (teremock)
50 sequential dispatches: ~30-60 seconds (server-per-dispatch)
```

### True Black-Box Testing

Test your bot the way users experience it. Send messages, click buttons, trigger commands — then verify the responses. No internal state manipulation, no implementation coupling.

### Multi-Step Conversations in One Test

Test complete user flows without juggling multiple test functions. The `update()` method lets you simulate follow-up messages, button clicks, and entire conversations in a single test.

### Zero Configuration

Works out of the box with `#[tokio::test]`. No custom thread builders, no special runtime configuration, no port management. Each `MockBot` gets its own server on a dynamically assigned port.

### Full Request Inspection

Access both the resulting message *and* the original bot request for detailed assertions:

```rust
let responses = bot.get_responses();

// Check the message that was sent
let msg = &responses.sent_messages_photo[0].message;
assert_eq!(msg.caption(), Some("Check out this image!"));

// Inspect the raw request your bot made
let request = &responses.sent_messages_photo[0].bot_request;
assert!(request.has_spoiler.unwrap_or(false));
```

## Features

- **Persistent mock server** — Server starts once per test, reuses across dispatches
- **Fluent builders** — `MockMessageText`, `MockCallbackQuery`, `MockMessagePhoto`, and more
- **Comprehensive API coverage** — 40+ Telegram Bot API methods supported
- **Type-safe responses** — Dedicated collections for each message type
- **Dependency injection** — Full support for `dptree::deps![]`
- **Dialogue state** — Works with `InMemStorage`, `RedisStorage`, or any teloxide storage
- **File operations** — Mock file uploads, downloads, and media groups
- **Stack-safe** — Each dispatch runs in its own tokio task with proper stack isolation

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
teremock = "0.5"
```

## Quick Start

### 1. Extract your handler tree

teremock tests your bot by running updates through your handler tree. Extract it into a separate function:

```rust
use teloxide::{
    dispatching::{UpdateFilterExt, UpdateHandler},
    prelude::*,
};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// Your handler function
async fn hello_world(bot: Bot, message: Message) -> HandlerResult {
    bot.send_message(message.chat.id, "Hello World!").await?;
    Ok(())
}

// Extract the handler tree into a function
pub fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    dptree::entry()
        .branch(Update::filter_message().endpoint(hello_world))
}

// Use it in your main dispatcher
#[tokio::main]
async fn main() {
    let bot = Bot::from_env();
    Dispatcher::builder(bot, handler_tree())
        .build()
        .dispatch()
        .await;
}
```

### 2. Write your tests

```rust
#[cfg(test)]
mod tests {
    use teremock::{MockBot, MockMessageText};
    use super::*;

    #[tokio::test]
    async fn test_hello_world() {
        // Create a mock message
        let mock_message = MockMessageText::new().text("Hi!");

        // Create a bot with your handler tree
        let mut bot = MockBot::new(mock_message, handler_tree()).await;

        // Dispatch the update
        bot.dispatch().await;

        // Check the response
        let responses = bot.get_responses();
        let message = responses.sent_messages.last().expect("No messages sent");
        assert_eq!(message.text(), Some("Hello World!"));
    }
}
```

### 3. Access detailed request information

For more specific assertions, use typed response collections:

```rust
#[tokio::test]
async fn test_with_request_details() {
    let mut bot = MockBot::new(MockMessageText::new().text("/photo"), handler_tree()).await;
    bot.dispatch().await;

    let responses = bot.get_responses();

    // sent_messages_text gives you both the message AND the original request
    let text_response = responses.sent_messages_text.last().unwrap();
    assert_eq!(text_response.message.text(), Some("Here's your photo!"));
    assert_eq!(text_response.bot_request.parse_mode, Some(ParseMode::Html));
}
```

### 4. Test multi-step conversations

Use `update()` to simulate follow-up messages in the same test. The mock server persists between dispatches, so you can test complete user flows:

```rust
#[tokio::test]
async fn test_conversation() {
    let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;

    // First message
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Welcome! Send me a number.")
    );

    // User sends a follow-up
    bot.update(MockMessageText::new().text("42"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("You sent: 42")
    );

    // Test callback queries too
    bot.update(MockCallbackQuery::new().data("confirm"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Confirmed!")
    );
}
```

## Working with Teloxide Dialogues

If your bot uses teloxide's dialogue system for stateful conversations, teremock has you covered. Just inject your storage as a dependency and test away.

### Setting up dialogue tests

```rust
use teloxide::{
    dispatching::{dialogue::InMemStorage, UpdateFilterExt, UpdateHandler},
    dptree::deps,
    prelude::*,
};
use teremock::{MockBot, MockMessageText};

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AwaitingName,
    AwaitingAge { name: String },
}

#[tokio::test]
async fn test_registration_flow() {
    let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;

    // Inject the storage — this is the key part for dialogues
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // Step 1: User sends /start
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Welcome! What's your name?")
    );

    // Step 2: User sends their name
    bot.update(MockMessageText::new().text("Alice"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Nice to meet you, Alice! How old are you?")
    );

    // Step 3: User sends their age
    bot.update(MockMessageText::new().text("25"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Registration complete! Alice, age 25.")
    );
}
```

The dialogue state transitions happen naturally through your handler tree — no need to manually set states. This is black-box testing at its finest.

## Supported Telegram API Methods

<details>
<summary>Click to expand full list</summary>

**Messages**
- `sendMessage`, `sendPhoto`, `sendVideo`, `sendAudio`, `sendVoice`
- `sendVideoNote`, `sendDocument`, `sendAnimation`, `sendSticker`
- `sendLocation`, `sendVenue`, `sendContact`, `sendPoll`, `sendDice`
- `sendInvoice`, `sendMediaGroup`, `sendChatAction`

**Editing**
- `editMessageText`, `editMessageCaption`, `editMessageReplyMarkup`

**Management**
- `deleteMessage`, `deleteMessages`, `forwardMessage`, `copyMessage`
- `pinChatMessage`, `unpinChatMessage`, `unpinAllChatMessages`

**Users & Moderation**
- `banChatMember`, `unbanChatMember`, `restrictChatMember`

**Callbacks & Commands**
- `answerCallbackQuery`, `setMessageReaction`, `setMyCommands`

**Files & Bot Info**
- `getFile`, `getMe`, `getUpdates`, `getWebhookInfo`

</details>

## Examples

The [`examples/`](examples/) directory contains complete bot implementations with tests:

| Example | Description |
|---------|-------------|
| [hello_world_bot](examples/hello_world_bot) | Simple message handling |
| [calculator_bot](examples/calculator_bot) | Dialogue state machine with callbacks |
| [deep_linking_bot](examples/deep_linking_bot) | Deep linking with command parameters |
| [album_bot](examples/album_bot) | Media group handling |
| [file_download_bot](examples/file_download_bot) | File upload and download operations |
| [phrase_bot](examples/phrase_bot) | Database integration patterns |

## Database Testing

teremock works seamlessly with database-backed bots. Use your preferred test isolation strategy:

```rust
#[tokio::test]
async fn test_with_database() {
    let pool = setup_test_database().await;  // Your test DB setup

    let mut bot = MockBot::new(MockMessageText::new().text("/save hello"), handler_tree()).await;
    bot.dependencies(deps![pool.clone()]);

    bot.dispatch().await;

    // Verify bot response
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Saved!")
    );

    // Verify database state
    let saved = sqlx::query!("SELECT phrase FROM phrases")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(saved.phrase, "hello");
}
```

## Acknowledgments

teremock builds upon the foundation laid by [teloxide_tests](https://github.com/LasterAlex/teloxide_tests) by LasterAlex. The original library pioneered the concept of mock testing for teloxide bots.

Key architectural changes in teremock:
- Persistent server architecture (15-30x faster test execution)
- Stack-safe dispatch isolation
- Black-box testing philosophy (no internal state manipulation)
- Async `MockBot::new()` for proper initialization

## License

MIT License — see [LICENSE](LICENSE) for details.
