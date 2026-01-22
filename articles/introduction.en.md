# Why Your Telegram Bot Tests Are Broken (And How to Fix Them)

Last week I deployed a bot to production and immediately got a screenshot from a user showing an error. The "Confirm Order" button was sending "Welcome!" instead of a confirmation. Classic.

I had "tested" the bot. Opened Telegram, clicked through the main flows, made sure `/start` worked. But that specific callback? I skipped it. Sound familiar?

After that incident I decided to figure out how to properly test teloxide bots. Spoiler: there weren't many good options. The existing solutions were either slow or required jumping through hoops. So I built my own library. Let me walk you through it.

## Table of Contents

1. [Three Ways to Test Bots (And Why They All Suck)](#three-ways-to-test-bots-and-why-they-all-suck)
2. [What is teremock and How Does It Work](#what-is-teremock-and-how-does-it-work)
3. [From Simple to Complex: Writing Your First Test](#from-simple-to-complex-writing-your-first-test)
4. [Testing Stateful Dialogues](#testing-stateful-dialogues)
5. [Advanced Techniques](#advanced-techniques)
6. [Performance: Why It's Fast](#performance-why-its-fast)
7. [Limitations (Yes, There Are Some)](#limitations-yes-there-are-some)

## Three Ways to Test Bots (And Why They All Suck)

Let's be honest about how people usually test Telegram bots.

### Method 1: "I Am My Own QA"

Open Telegram, message the bot, click buttons, see what happens. If it doesn't crash, ship it.

The problem is that this isn't testing. It's hope. Hope that you didn't forget some edge case. Hope that your refactoring didn't break anything. Hope that the "quick fix" you pushed at 3 AM didn't miss something obvious.

I had a bot for processing applications. 47 different dialogue states, tons of callback buttons, input validation everywhere. Manual testing took 15-20 minutes each time. After the third refactoring I just stopped checking the "unimportant" branches. Guess where the bugs ended up.

### Method 2: Tests Against the Real API

Logical idea: create a test bot, get a token, write tests that actually hit Telegram's servers.

```rust
#[tokio::test]
async fn test_start_command() {
    let bot = Bot::new("YOUR_TEST_TOKEN");
    // How do you send a message to yourself?
    // How do you get the bot's response?
    // How do you wait for it?
}
```

This is where the problems start:

**Network dependency.** Tests fail due to timeouts, flaky internet, or issues on Telegram's end. Flaky tests aren't tests. They're false positive generators.

**Rate limits.** Telegram throttles requests. Run 50 tests in parallel and you get banned for a minute. Run them sequentially and you're waiting 5 minutes for them to finish.

**CI/CD headaches.** You need to store tokens in secrets, configure network access from runners, handle random failures. Every other pipeline will be yellow.

**Speed.** Each HTTP request adds 50-200ms of latency. A test with 10 interactions takes seconds. A full test suite takes minutes.

### Method 3: Write Your Own Mocks

You could write a mock server that pretends to be the Telegram API. In theory.

In practice this means:
- Implementing an HTTP server
- Emulating Telegram's response structures (lots of edge cases there)
- Supporting all the API methods your bot uses
- Updating mocks when the API changes

That's weeks of work. For most projects it's not worth it.

### The Root of the Problem

Why is this so hard? It comes down to teloxide's architecture.

A bot isn't just a collection of functions. It's a handler that needs a `Bot` object to send responses. And `Bot` is an HTTP client that talks to the real API. You can't just call a handler function in a test because it needs the full context.

```rust
async fn handle_start(bot: Bot, msg: Message) -> HandlerResult {
    // bot.send_message() is an HTTP request to api.telegram.org
    bot.send_message(msg.chat.id, "Hello!").await?;
    Ok(())
}
```

You need a way to replace `api.telegram.org` with something local while keeping all the other teloxide infrastructure intact: dispatching, dependency injection, dialogue management.

## What is teremock and How Does It Work

**teremock** (**Te**legram · **Re**alistic · **Mock**ing) is a library that solves this problem. It spins up a local HTTP server that mimics the Telegram Bot API and redirects the `Bot` object to use it.

```
┌─────────────────┐                     ┌─────────────────┐
│                 │  POST /sendMessage  │                 │
│    Your bot     │ ──────────────────▶ │    teremock     │
│   (teloxide)    │                     │ localhost:XXXX  │
│                 │ ◀────────────────── │                 │
└─────────────────┘  {"ok": true, ...}  └─────────────────┘
        │                                       │
        │ thinks it's talking                   │ records
        │ to Telegram                           │ all requests
        ▼                                       ▼
┌─────────────────┐                     ┌─────────────────┐
│  handler_tree() │                     │ get_responses() │
│   your logic    │                     │  for assertions │
└─────────────────┘                     └─────────────────┘
```

Key features:

**Persistent server.** The server starts once when the test begins and gets reused for all dispatches. This is critical for performance.

**Black-box testing.** You interact with the bot the same way a user would: send messages, click buttons, check responses. There's no way to directly manipulate dialogue state. Everything goes through the handlers.

**Full type safety.** Everything is written in Rust using teloxide's types. The compiler won't let you pass invalid data.

**40+ API methods.** sendMessage, sendPhoto, editMessageText, answerCallbackQuery, and everything else you need for most bots.

## From Simple to Complex: Writing Your First Test

Let's start with a minimal example. Here's a bot that replies "Hello World!" to any message:

```rust
use teloxide::{
    dispatching::{UpdateFilterExt, UpdateHandler},
    prelude::*,
};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn hello_world(bot: Bot, message: Message) -> HandlerResult {
    bot.send_message(message.chat.id, "Hello World!").await?;
    Ok(())
}

// Important: handler tree MUST be extracted into a separate function
pub fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    dptree::entry().branch(Update::filter_message().endpoint(hello_world))
}
```

Note that `handler_tree()` is a separate function. This isn't optional. In `main()` you pass it to `Dispatcher::builder()`, in tests you pass it to `MockBot::new()`. Same code, different contexts.

Now the test:

```rust
use teremock::{MockBot, MockMessageText};

#[tokio::test]
async fn test_hello_world() {
    // 1. Create a mock message
    let mock_message = MockMessageText::new().text("Hi!");

    // 2. Create MockBot with our handler tree
    let mut bot = MockBot::new(mock_message, handler_tree()).await;

    // 3. Run the message through the handlers
    bot.dispatch().await;

    // 4. Check the result
    let responses = bot.get_responses();
    let message = responses.sent_messages.last().expect("Bot didn't send anything");
    assert_eq!(message.text(), Some("Hello World!"));
}
```

What's happening under the hood:

1. `MockBot::new()` starts an actix-web server on a random free port
2. A `Bot` from teloxide is created, configured to use this local server
3. A `Dispatcher` is created with your handler tree
4. `dispatch()` sends the mock message into the dispatcher
5. The bot calls `send_message()`, the request goes to the mock server
6. The mock server records the request and returns a valid response
7. `get_responses()` gives you access to all recorded requests

The test runs in milliseconds. No network. No tokens. Deterministic.

## Testing Stateful Dialogues

Hello World is a toy example. Real bots have state. Let's look at a calculator bot:

```rust
#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AwaitingFirstNumber { operation: String },
    AwaitingSecondNumber { operation: String, first: i64 },
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;
```

User flow:
1. Sends `/start`
2. Sees "Add" and "Subtract" buttons
3. Clicks "Add"
4. Gets prompted for the first number
5. Enters "5"
6. Gets prompted for the second number
7. Enters "4"
8. Gets "Result: 9"

How do you test this? Like this:

```rust
use teremock::{MockBot, MockCallbackQuery, MockMessageText};
use teloxide::dptree::deps;

#[tokio::test]
async fn test_full_addition_flow() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/start"),
        handler_tree()
    ).await;

    // Inject storage for dialogues
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // Step 1: /start
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("What would you like to do?")
    );

    // Step 2: click the button
    bot.update(MockCallbackQuery::new().data("add"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the first number")
    );

    // Step 3: enter the first number
    bot.update(MockMessageText::new().text("5"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the second number")
    );

    // Step 4: enter the second number
    bot.update(MockMessageText::new().text("4"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Result: 9")
    );
}
```

A few important points:

**`bot.update()` replaces the current message.** You don't need to create a new MockBot for each step. The server is persistent, state is preserved between dispatches.

**`dependencies()` works like in production.** `InMemStorage` is the real storage from teloxide. In tests you can use the same one, or substitute a mock database.

**State changes through handlers.** We don't call `set_state(State::AwaitingFirstNumber)`. State changes naturally, just like it would for a real user.

### Testing Error Handling

A good bot handles bad input gracefully. Let's verify that:

```rust
#[tokio::test]
async fn test_invalid_input_handling() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/start"),
        handler_tree()
    ).await;
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // Get to the number input state
    bot.dispatch().await;
    bot.update(MockCallbackQuery::new().data("add"));
    bot.dispatch().await;

    // User enters garbage
    bot.update(MockMessageText::new().text("not a number"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Please enter a number")
    );

    // User sends a photo for some reason
    bot.update(MockMessagePhoto::new());
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Please send a text message")
    );

    // User finally gets it right
    bot.update(MockMessageText::new().text("5"));
    bot.dispatch().await;
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Enter the second number")
    );
}
```

Three scenarios in one test: invalid text, wrong message type, recovery. This kind of test is way more valuable than three isolated unit tests.

## Advanced Techniques

### Request Inspection

Sometimes you need to verify more than just the response text. For example, whether the bot used HTML formatting:

```rust
#[tokio::test]
async fn test_message_formatting() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/formatted"),
        handler_tree()
    ).await;

    bot.dispatch().await;

    // sent_messages_text gives you access to the original request
    let response = bot.get_responses().sent_messages_text.last().unwrap();

    // Check the text
    assert_eq!(response.message.text(), Some("<b>Bold</b> text"));

    // Check parse_mode in the request
    assert_eq!(response.bot_request.parse_mode, Some(ParseMode::Html));
}
```

For media this is even more useful:

```rust
#[tokio::test]
async fn test_photo_with_spoiler() {
    let mut bot = MockBot::new(
        MockMessageText::new().text("/secret"),
        handler_tree()
    ).await;

    bot.dispatch().await;

    let photo = bot.get_responses().sent_messages_photo.last().unwrap();

    // Check caption
    assert_eq!(photo.message.caption(), Some("Secret!"));

    // Check spoiler flag
    assert!(photo.bot_request.has_spoiler.unwrap_or(false));
}
```

The `get_responses()` struct contains typed collections:
- `sent_messages` for all messages (just `Message`)
- `sent_messages_text` for text messages with access to `bot_request`
- `sent_messages_photo` for photos with access to `bot_request`
- `sent_messages_video`, `sent_messages_document`, etc.

### Customizing Mock Objects

Builders let you configure any field:

```rust
// Message from a specific user
let msg = MockMessageText::new()
    .text("Hello from Alex")
    .from(MockUser::new()
        .id(12345)
        .first_name("Alex")
        .username("alex")
        .build());

// Callback with a specific message_id
let callback = MockCallbackQuery::new()
    .data("confirm")
    .message_id(42);

// Photo with multiple sizes
let photo = MockMessagePhoto::new()
    .caption("Nice photo")
    .file_id("AgACAgIAAxk...", "unique_id");
```

### Database Integration

teremock doesn't get in the way of real dependencies:

```rust
#[tokio::test]
async fn test_save_to_database() {
    // Set up test DB (testcontainers, in-memory SQLite, whatever)
    let pool = setup_test_database().await;

    let mut bot = MockBot::new(
        MockMessageText::new().text("/save important info"),
        handler_tree()
    ).await;

    // Inject the pool as a dependency
    bot.dependencies(deps![pool.clone()]);

    bot.dispatch().await;

    // Check bot response
    assert_eq!(
        bot.get_responses().sent_messages.last().unwrap().text(),
        Some("Saved!")
    );

    // Verify data is actually in the DB
    let saved = sqlx::query!("SELECT content FROM notes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(saved.content, "important info");
}
```

## Performance: Why It's Fast

The main optimization is the persistent server. Compare:

| Approach | 50 dispatches |
|----------|---------------|
| New server per dispatch | 30-60 seconds |
| teremock (persistent server) | ~2 seconds |

**15-30x speedup.**

Where does the difference come from? Starting an HTTP server is expensive. You need to bind a port, initialize handlers, warm up caches. Do that 50 times and it adds up.

teremock starts the server once. All subsequent `dispatch()` and `update()` calls use the already running server.

Additional optimizations:

**Stack-safe execution.** Each `dispatch()` runs in its own tokio task with its own stack. You can run hundreds of dispatches in a single test without stack overflow.

**Automatic port management.** Each `MockBot` gets its own port. You can run tests in parallel without conflicts.

**Minimal allocations.** Mock responses are created lazily, only when needed.

On a real project with 50+ tests the full suite runs in 10-15 seconds. That's fast enough to run after every file save.

## Supported API Methods

teremock supports out of the box:

**Sending messages:**
sendMessage, sendPhoto, sendVideo, sendAudio, sendVoice, sendVideoNote, sendDocument, sendAnimation, sendSticker, sendLocation, sendVenue, sendContact, sendPoll, sendDice, sendInvoice, sendMediaGroup, sendChatAction

**Editing:**
editMessageText, editMessageCaption, editMessageReplyMarkup

**Management:**
deleteMessage, deleteMessages, forwardMessage, copyMessage, pinChatMessage, unpinChatMessage, unpinAllChatMessages

**Moderation:**
banChatMember, unbanChatMember, restrictChatMember

**Other:**
answerCallbackQuery, setMessageReaction, setMyCommands, getFile, getMe, getUpdates, getWebhookInfo

If a method isn't implemented, the library returns a clear error telling you which endpoint is missing.

## Limitations (Yes, There Are Some)

It wouldn't be fair to skip the limitations.

**Async constructor.** `MockBot::new()` is an async function. You need `#[tokio::test]` or another async runtime. This isn't a bug, it's a consequence of needing to start an HTTP server.

**Black-box only.** There's no `get_dialogue_state()` or `set_dialogue_state()`. This is intentional. If you need to manipulate state directly, you might want to reconsider your test architecture.

**Not all API methods.** 40+ is a lot, but the Telegram Bot API has more. Inline mode, payments, some admin methods might not be implemented.

**Localhost only.** The mock server runs locally. Distributed testing isn't supported (though that's a rare scenario for bots anyway).

## Quick Start

```toml
[dev-dependencies]
teremock = "0.5"
```

```rust
#[cfg(test)]
mod tests {
    use teremock::{MockBot, MockMessageText};
    use super::handler_tree;

    #[tokio::test]
    async fn test_my_bot() {
        let mut bot = MockBot::new(
            MockMessageText::new().text("/start"),
            handler_tree()
        ).await;

        bot.dispatch().await;

        let responses = bot.get_responses();
        assert!(!responses.sent_messages.is_empty());
    }
}
```

```bash
cargo test
```

## Examples

The repository has an `examples/` directory with complete bots:

| Example | What it demonstrates |
|---------|---------------------|
| `hello_world_bot` | Basic message handling |
| `calculator_bot` | Stateful dialogues, callback buttons |
| `deep_linking_bot` | Deep linking with parameters |
| `album_bot` | Media group handling |
| `file_download_bot` | File operations |
| `phrase_bot` | Database integration |

Each example includes both bot code and tests. I recommend starting with `calculator_bot` since it covers most typical scenarios.

## Links

- **GitHub:** [https://github.com/zerosixty/teremock](https://github.com/zerosixty/teremock)
- **Crates.io:** [https://crates.io/crates/teremock](https://crates.io/crates/teremock)
- **Documentation:** [https://docs.rs/teremock](https://docs.rs/teremock)

---

## Acknowledgments

The idea of mock testing for teloxide bots isn't new. The [teloxide_tests](https://github.com/LasterAlex/teloxide_tests) project by LasterAlex pioneered this approach and was a major source of inspiration for teremock's architecture.

Special thanks to the [teloxide](https://github.com/teloxide/teloxide) team for building such a great framework. Without their work the Rust Telegram bot ecosystem would look very different.

---

*If this was helpful, consider starring the repo on GitHub. Found a bug or missing an API method? Open an issue and we'll figure it out.*
