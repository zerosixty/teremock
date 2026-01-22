# How I Stopped Worrying and Started Testing My Telegram Bots

*A story about testing Telegram bots without the pain*

---

Have you ever shipped a Telegram bot and immediately regretted it? Maybe your `/start` command crashed spectacularly at 3 AM, or that callback button you "definitely tested" decided to ghost your users. I've been there. Testing Telegram bots traditionally meant one of two things: manually clicking through your bot like a QA intern, or setting up elaborate integration tests that require actual API tokens and network access.

Neither is fun. Neither scales. And both make CI pipelines cry.

That's why I built **teremock** — a testing library that lets you write fast, reliable tests for your [teloxide](https://github.com/teloxide/teloxide) bots without ever hitting the real Telegram API.

Let me show you what I mean.

## The Problem with Testing Telegram Bots

Picture this: you've got a calculator bot. Users send `/start`, click a button to add or subtract, enter two numbers, and get a result. Simple enough. But how do you test it?

**Option 1: Manual testing.** You open Telegram, type commands, click buttons, and hope everything works. Rinse and repeat after every code change. This doesn't scale.

**Option 2: Real API testing.** You set up a test bot token, hit the actual Telegram servers, and pray your internet is stable. Tests take forever because network requests aren't exactly speedy. Good luck running this in CI without exposing credentials.

**Option 3: Mock everything yourself.** You spend more time building test infrastructure than actual features. Eventually, you question your life choices.

There had to be a better way.

## Enter teremock

teremock (**Te**legram · **Re**alistic · **Mock**ing) takes a different approach. It spins up a lightweight mock server that pretends to be the Telegram Bot API. Your bot talks to this server instead of the real one. From your bot's perspective, nothing changes — it's making the same API calls it always does. But now those calls are instant, offline, and completely under your control.

Here's the simplest possible test:

```rust
use teremock::{MockBot, MockMessageText};

#[tokio::test]
async fn test_hello_world() {
    // Create a mock message (as if a user sent "Hi!")
    let mock_message = MockMessageText::new().text("Hi!");

    // Create a bot with your handler tree
    let mut bot = MockBot::new(mock_message, handler_tree()).await;

    // Dispatch the update through your handlers
    bot.dispatch().await;

    // Check what your bot sent back
    let responses = bot.get_responses();
    assert_eq!(
        responses.sent_messages.last().unwrap().text(),
        Some("Hello World!")
    );
}
```

That's it. No API tokens. No network. No waiting. Just fast, deterministic tests.

## Let's Build Something Real

Enough theory. Let's test an actual stateful bot — a simple calculator that walks users through adding or subtracting numbers.

First, here's the handler setup (the part you'd normally write anyway):

```rust
use teloxide::{
    dispatching::{dialogue::InMemStorage, UpdateFilterExt, UpdateHandler},
    dptree::deps,
    prelude::*,
};

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AwaitingFirstNumber { operation: String },
    AwaitingSecondNumber { operation: String, first: i64 },
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;

fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    dptree::entry()
        .branch(Update::filter_message().enter_dialogue::<Message, InMemStorage<State>, State>()
            // ... your handler branches here
        )
}
```

Now the fun part — testing the entire conversation flow in one test:

```rust
use teremock::{MockBot, MockCallbackQuery, MockMessageText};
use teloxide::dptree::deps;

#[tokio::test]
async fn test_full_addition_flow() {
    // Start with /start command
    let mut bot = MockBot::new(
        MockMessageText::new().text("/start"),
        handler_tree()
    ).await;

    // Inject your storage dependency
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // User sends /start
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("What do you want to do?")
    );

    // User clicks the "add" button
    bot.update(MockCallbackQuery::new().data("add"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the first number")
    );

    // User enters first number
    bot.update(MockMessageText::new().text("5"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the second number")
    );

    // User enters second number
    bot.update(MockMessageText::new().text("4"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Your result: 9")
    );
}
```

Notice what's happening here:
- **One test, full conversation.** No need to split your flow into five separate tests.
- **Natural state transitions.** The dialogue state updates through your actual handlers, not manual manipulation.
- **Real dependency injection.** Your `InMemStorage` works exactly like in production.

## What About Edge Cases?

Great bots handle weird inputs gracefully. Let's test that:

```rust
#[tokio::test]
async fn test_invalid_number_input() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/start"),
        handler_tree()
    ).await;
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // Get to the "enter first number" state
    bot.dispatch().await;
    bot.update(MockCallbackQuery::new().data("add"));
    bot.dispatch().await;

    // User sends garbage instead of a number
    bot.update(MockMessageText::new().text("not a number"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Please enter a valid number")
    );

    // User sends a photo for some reason
    bot.update(MockMessagePhoto::new());
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Please send text")
    );

    // User finally sends a valid number
    bot.update(MockMessageText::new().text("5"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the second number")
    );
}
```

This test covers three scenarios in one function: invalid text, wrong message type, and recovery. Your error handling actually gets tested.

## Digging Deeper: Request Inspection

Sometimes you need to verify more than just the message text. Maybe you're testing that your bot uses the right parse mode, or that a photo is marked as a spoiler. teremock gives you full access to both the sent message *and* the original request:

```rust
#[tokio::test]
async fn test_message_formatting() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/styled"),
        handler_tree()
    ).await;

    bot.dispatch().await;

    let responses = bot.get_responses();

    // Check the message content
    let response = &responses.sent_messages_text.last().unwrap();
    assert_eq!(response.message.text(), Some("<b>Bold</b> text"));

    // Verify the parse mode in the original request
    assert_eq!(response.bot_request.parse_mode, Some(ParseMode::Html));
}
```

For media messages, this becomes even more useful:

```rust
#[tokio::test]
async fn test_photo_with_spoiler() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/secret_photo"),
        handler_tree()
    ).await;

    bot.dispatch().await;

    let photo = &bot.get_responses().sent_messages_photo.last().unwrap();
    assert_eq!(photo.message.caption(), Some("Mystery image!"));
    assert!(photo.bot_request.has_spoiler.unwrap_or(false));
}
```

## The Performance Story

Here's where teremock really shines. The mock server starts once when you create a `MockBot` and persists across all your dispatches. No server restart between interactions.

The numbers speak for themselves:

| Scenario | teremock | Server-per-dispatch |
|----------|----------|---------------------|
| 50 sequential dispatches | ~2 seconds | ~30-60 seconds |

That's **15-30x faster** for comprehensive test suites. And because each dispatch runs in its own tokio task, you won't hit stack overflow issues even with dozens of interactions in a single test.

Your CI pipeline will thank you.

## What's Under the Hood?

teremock supports 40+ Telegram Bot API methods out of the box:

**Messages:** sendMessage, sendPhoto, sendVideo, sendAudio, sendVoice, sendDocument, sendAnimation, sendSticker, sendLocation, sendVenue, sendContact, sendPoll, sendDice, sendInvoice, sendMediaGroup, sendChatAction...

**Editing:** editMessageText, editMessageCaption, editMessageReplyMarkup

**Management:** deleteMessage, forwardMessage, copyMessage, pinChatMessage, unpinChatMessage...

**Callbacks & More:** answerCallbackQuery, setMessageReaction, setMyCommands, getFile, getMe...

All the builders follow a fluent pattern:

```rust
// Text message with custom sender
let msg = MockMessageText::new()
    .text("Hello from a specific user")
    .from(MockUser::new().id(12345).first_name("Alex").build());

// Callback query with specific data
let query = MockCallbackQuery::new()
    .data("button_clicked")
    .from(MockUser::new().id(12345).build());

// Photo message
let photo = MockMessagePhoto::new()
    .caption("Check this out!");
```

## Getting Started

Add teremock to your dev dependencies:

```toml
[dev-dependencies]
teremock = "0.5"
```

And you're ready to go. Works with `#[tokio::test]` out of the box.

**Links:**# How I Stopped Worrying and Started Testing My Telegram Bots

*A story about testing Telegram bots without the pain*

---

Have you ever shipped a Telegram bot and immediately regretted it? Maybe your `/start` command crashed spectacularly at 3 AM, or that callback button you "definitely tested" decided to ghost your users. I've been there. Testing Telegram bots traditionally meant one of two things: manually clicking through your bot like a QA intern, or setting up elaborate integration tests that require actual API tokens and network access.

Neither is fun. Neither scales. And both make CI pipelines cry.

That's why I built **teremock** — a testing library that lets you write fast, reliable tests for your [teloxide](https://github.com/teloxide/teloxide) bots without ever hitting the real Telegram API.

Let me show you what I mean.

## The Problem with Testing Telegram Bots

Picture this: you've got a calculator bot. Users send `/start`, click a button to add or subtract, enter two numbers, and get a result. Simple enough. But how do you test it?

**Option 1: Manual testing.** You open Telegram, type commands, click buttons, and hope everything works. Rinse and repeat after every code change. This doesn't scale.

**Option 2: Real API testing.** You set up a test bot token, hit the actual Telegram servers, and pray your internet is stable. Tests take forever because network requests aren't exactly speedy. Good luck running this in CI without exposing credentials.

**Option 3: Mock everything yourself.** You spend more time building test infrastructure than actual features. Eventually, you question your life choices.

There had to be a better way.

## Enter teremock

teremock (**Te**legram · **Re**alistic · **Mock**ing) takes a different approach. It spins up a lightweight mock server that pretends to be the Telegram Bot API. Your bot talks to this server instead of the real one. From your bot's perspective, nothing changes — it's making the same API calls it always does. But now those calls are instant, offline, and completely under your control.

Here's the simplest possible test:

```rust
use teremock::{MockBot, MockMessageText};

#[tokio::test]
async fn test_hello_world() {
    // Create a mock message (as if a user sent "Hi!")
    let mock_message = MockMessageText::new().text("Hi!");

    // Create a bot with your handler tree
    let mut bot = MockBot::new(mock_message, handler_tree()).await;

    // Dispatch the update through your handlers
    bot.dispatch().await;

    // Check what your bot sent back
    let responses = bot.get_responses();
    assert_eq!(
        responses.sent_messages.last().unwrap().text(),
        Some("Hello World!")
    );
}
```

That's it. No API tokens. No network. No waiting. Just fast, deterministic tests.

## Let's Build Something Real

Enough theory. Let's test an actual stateful bot — a simple calculator that walks users through adding or subtracting numbers.

First, here's the handler setup (the part you'd normally write anyway):

```rust
use teloxide::{
    dispatching::{dialogue::InMemStorage, UpdateFilterExt, UpdateHandler},
    dptree::deps,
    prelude::*,
};

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AwaitingFirstNumber { operation: String },
    AwaitingSecondNumber { operation: String, first: i64 },
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;

fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    dptree::entry()
        .branch(Update::filter_message().enter_dialogue::<Message, InMemStorage<State>, State>()
            // ... your handler branches here
        )
}
```

Now the fun part — testing the entire conversation flow in one test:

```rust
use teremock::{MockBot, MockCallbackQuery, MockMessageText};
use teloxide::dptree::deps;

#[tokio::test]
async fn test_full_addition_flow() {
    // Start with /start command
    let mut bot = MockBot::new(
        MockMessageText::new().text("/start"),
        handler_tree()
    ).await;

    // Inject your storage dependency
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // User sends /start
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("What do you want to do?")
    );

    // User clicks the "add" button
    bot.update(MockCallbackQuery::new().data("add"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the first number")
    );

    // User enters first number
    bot.update(MockMessageText::new().text("5"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the second number")
    );

    // User enters second number
    bot.update(MockMessageText::new().text("4"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Your result: 9")
    );
}
```

Notice what's happening here:
- **One test, full conversation.** No need to split your flow into five separate tests.
- **Natural state transitions.** The dialogue state updates through your actual handlers, not manual manipulation.
- **Real dependency injection.** Your `InMemStorage` works exactly like in production.

## What About Edge Cases?

Great bots handle weird inputs gracefully. Let's test that:

```rust
#[tokio::test]
async fn test_invalid_number_input() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/start"),
        handler_tree()
    ).await;
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // Get to the "enter first number" state
    bot.dispatch().await;
    bot.update(MockCallbackQuery::new().data("add"));
    bot.dispatch().await;

    // User sends garbage instead of a number
    bot.update(MockMessageText::new().text("not a number"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Please enter a valid number")
    );

    // User sends a photo for some reason
    bot.update(MockMessagePhoto::new());
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Please send text")
    );

    // User finally sends a valid number
    bot.update(MockMessageText::new().text("5"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the second number")
    );
}
```

This test covers three scenarios in one function: invalid text, wrong message type, and recovery. Your error handling actually gets tested.

## Digging Deeper: Request Inspection

Sometimes you need to verify more than just the message text. Maybe you're testing that your bot uses the right parse mode, or that a photo is marked as a spoiler. teremock gives you full access to both the sent message *and* the original request:

```rust
#[tokio::test]
async fn test_message_formatting() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/styled"),
        handler_tree()
    ).await;

    bot.dispatch().await;

    let responses = bot.get_responses();

    // Check the message content
    let response = &responses.sent_messages_text.last().unwrap();
    assert_eq!(response.message.text(), Some("<b>Bold</b> text"));

    // Verify the parse mode in the original request
    assert_eq!(response.bot_request.parse_mode, Some(ParseMode::Html));
}
```

For media messages, this becomes even more useful:

```rust
#[tokio::test]
async fn test_photo_with_spoiler() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/secret_photo"),
        handler_tree()
    ).await;

    bot.dispatch().await;

    let photo = &bot.get_responses().sent_messages_photo.last().unwrap();
    assert_eq!(photo.message.caption(), Some("Mystery image!"));
    assert!(photo.bot_request.has_spoiler.unwrap_or(false));
}
```

## The Performance Story

Here's where teremock really shines. The mock server starts once when you create a `MockBot` and persists across all your dispatches. No server restart between interactions.

The numbers speak for themselves:

| Scenario | teremock | Server-per-dispatch |
|----------|----------|---------------------|
| 50 sequential dispatches | ~2 seconds | ~30-60 seconds |

That's **15-30x faster** for comprehensive test suites. And because each dispatch runs in its own tokio task, you won't hit stack overflow issues even with dozens of interactions in a single test.

Your CI pipeline will thank you.

## What's Under the Hood?

teremock supports 40+ Telegram Bot API methods out of the box:

**Messages:** sendMessage, sendPhoto, sendVideo, sendAudio, sendVoice, sendDocument, sendAnimation, sendSticker, sendLocation, sendVenue, sendContact, sendPoll, sendDice, sendInvoice, sendMediaGroup, sendChatAction...

**Editing:** editMessageText, editMessageCaption, editMessageReplyMarkup

**Management:** deleteMessage, forwardMessage, copyMessage, pinChatMessage, unpinChatMessage...

**Callbacks & More:** answerCallbackQuery, setMessageReaction, setMyCommands, getFile, getMe...

All the builders follow a fluent pattern:

```rust
// Text message with custom sender
let msg = MockMessageText::new()
    .text("Hello from a specific user")
    .from(MockUser::new().id(12345).first_name("Alex").build());

// Callback query with specific data
let query = MockCallbackQuery::new()
    .data("button_clicked")
    .from(MockUser::new().id(12345).build());

// Photo message
let photo = MockMessagePhoto::new()
    .caption("Check this out!");
```

## Getting Started

Add teremock to your dev dependencies:

```toml
[dev-dependencies]
teremock = "0.5"
```

And you're ready to go. Works with `#[tokio::test]` out of the box.

**Links:**
- GitHub: [https://github.com/zerosixty/teremock](https://github.com/zerosixty/teremock)
- Crates.io: [https://crates.io/crates/teremock](https://crates.io/crates/teremock)
- Documentation: [https://docs.rs/teremock](https://docs.rs/teremock)

The repository includes several example bots with full test suites:
- `hello_world_bot` — The basics
- `calculator_bot` — Stateful dialogues with callbacks
- `album_bot` — Media group handling
- `file_download_bot` — File operations
- `phrase_bot` — Database integration patterns

## Wrapping Up

Testing Telegram bots doesn't have to be painful. With teremock, you can:

- Write tests that run in milliseconds, not minutes
- Test complete multi-step conversations in single test functions
- Verify your bot's behavior without network access or API tokens
- Catch edge cases before your users do

The days of manual Telegram testing or flaky network-dependent CI are over.

---

## Acknowledgments

teremock builds upon ideas from [teloxide_tests](https://github.com/LasterAlex/teloxide_tests) by LasterAlex, which pioneered the concept of mock testing for teloxide bots. That project was a major source of inspiration for this library's approach.

A huge thank you to the [teloxide](https://github.com/teloxide/teloxide) team for building such an excellent Telegram bot framework. Their work makes building Telegram bots in Rust an absolute joy.

---

*Happy testing!*

- GitHub: [https://github.com/zerosixty/teremock](https://github.com/zerosixty/teremock)
- Crates.io: [https://crates.io/crates/teremock](https://crates.io/crates/teremock)
- Documentation: [https://docs.rs/teremock](https://docs.rs/teremock)

The repository includes several example bots with full test suites:
- `hello_world_bot` — The basics
- `calculator_bot` — Stateful dialogues with callbacks
- `album_bot` — Media group handling
- `file_download_bot` — File operations
- `phrase_bot` — Database integration patterns

## Wrapping Up

Testing Telegram bots doesn't have to be painful. With teremock, you can:

- Write tests that run in milliseconds, not minutes
- Test complete multi-step conversations in single test functions
- Verify your bot's behavior without network access or API tokens
- Catch edge cases before your users do

The days of manual Telegram testing or flaky network-dependent CI are over.

---

## Acknowledgments

teremock builds upon ideas from [teloxide_tests](https://github.com/LasterAlex/teloxide_tests) by LasterAlex, which pioneered the concept of mock testing for teloxide bots. That project was a major source of inspiration for this library's approach.

A huge thank you to the [teloxide](https://github.com/teloxide/teloxide) team for building such an excellent Telegram bot framework. Their work makes building Telegram bots in Rust an absolute joy.

---

*Happy testing!*
