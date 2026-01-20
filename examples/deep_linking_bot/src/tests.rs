use teloxide::{dispatching::dialogue::InMemStorage, dptree::deps};
use teremock::{MockBot, MockMessagePhoto, MockMessageText};

use crate::{add_deep_link, handler_tree::handler_tree, text, State};

/// Test regular /start command (no deep link)
#[tokio::test]
async fn test_start() {
    let mock_message = MockMessageText::new().text("/start");
    let mut bot = MockBot::new(mock_message.clone(), handler_tree()).await;

    bot.dependencies(deps![InMemStorage::<State>::new()]);
    let me = bot.me.clone();

    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(
        last_msg.text(),
        Some(add_deep_link(text::START, me, mock_message.chat.id).as_str())
    );
}

/// Test deep link flow - when user clicks t.me/bot?start=987654321
#[tokio::test]
async fn test_with_deep_link() {
    // Because https://t.me/some_bot?start=987654321 is the same as sending "/start 987654321",
    // we can simulate it with this
    let mock_message = MockMessageText::new().text("/start 987654321");
    let mut bot = MockBot::new(mock_message, handler_tree()).await;

    bot.dependencies(deps![InMemStorage::<State>::new()]);

    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(last_msg.text(), Some(text::SEND_YOUR_MESSAGE));
}

/// Test the full flow: deep link -> send message -> verify delivery
/// This is a black-box test that drives through the complete user interaction
#[tokio::test]
async fn test_send_message_flow() {
    // Step 1: User clicks deep link to message user 987654321
    let mock_message = MockMessageText::new().text("/start 987654321");
    let mut bot = MockBot::new(mock_message.clone(), handler_tree()).await;

    let me = bot.me.clone();
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    // User arrives via deep link
    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(last_msg.text(), Some(text::SEND_YOUR_MESSAGE));

    // Step 2: User sends the message they want to forward
    bot.update(MockMessageText::new().text("I love you!"));
    bot.dispatch().await;

    let responses = bot.get_responses();

    // This is the message that was sent to 987654321. It is always first after dispatch
    let sent_message = responses.sent_messages[0].clone();
    // And this is the confirmation message sent to the sender
    let response_message = responses.sent_messages[1].clone();

    assert_eq!(
        sent_message.text().unwrap(),
        text::YOU_HAVE_A_NEW_MESSAGE.replace("{message}", "I love you!")
    );
    assert_eq!(sent_message.chat.id.0, 987654321);

    assert_eq!(
        response_message.text().unwrap(),
        add_deep_link(text::MESSAGE_SENT, me, mock_message.chat.id)
    );
    assert_eq!(response_message.chat.id, mock_message.chat.id);
}

/// Test wrong deep link format
#[tokio::test]
async fn test_wrong_link() {
    let mock_message = MockMessageText::new().text("/start not_id");
    let mut bot = MockBot::new(mock_message, handler_tree()).await;
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(last_msg.text(), Some(text::WRONG_LINK));
}

/// Test that sending non-text (photo) when expecting text is handled correctly
#[tokio::test]
async fn test_not_a_text() {
    // First arrive via deep link
    let mock_message = MockMessageText::new().text("/start 987654321");
    let mut bot = MockBot::new(mock_message, handler_tree()).await;
    bot.dependencies(deps![InMemStorage::<State>::new()]);

    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(last_msg.text(), Some(text::SEND_YOUR_MESSAGE));

    // Then send a photo instead of text
    bot.update(MockMessagePhoto::new());
    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(last_msg.text(), Some(text::SEND_TEXT));
}
