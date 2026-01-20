use teloxide::{
    dispatching::dialogue::{InMemStorage, Storage},
    dptree::deps,
};
use teremock::{MockBot, MockCallbackQuery, MockMessagePhoto, MockMessageText};

use crate::{handler_tree::handler_tree, text, MyStorage, State};

/// Creates an in-memory storage for tests (no Redis required)
fn get_test_storage() -> MyStorage {
    InMemStorage::<State>::new().erase()
}

/// Helper to dispatch and check the last response text
async fn dispatch_and_check(bot: &mut MockBot<Box<dyn std::error::Error + Send + Sync + 'static>, teremock::DistributionKey>, expected: &str) {
    bot.dispatch().await;
    let responses = bot.get_responses();
    let last_msg = responses.sent_messages.last().expect("No messages sent");
    assert_eq!(last_msg.text(), Some(expected), "Message text mismatch");
}

/// Test the complete addition flow from start to result
/// This is a black-box test that simulates a real user interaction
#[tokio::test]
async fn test_full_addition_flow() {
    // Start the conversation
    let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
    bot.dependencies(deps![get_test_storage()]);

    // User sends /start
    dispatch_and_check(&mut bot, text::WHAT_DO_YOU_WANT).await;

    // User clicks "add" callback
    bot.update(MockCallbackQuery::new().data("add"));
    dispatch_and_check(&mut bot, text::ENTER_THE_FIRST_NUMBER).await;

    // User enters first number
    bot.update(MockMessageText::new().text("5"));
    dispatch_and_check(&mut bot, text::ENTER_THE_SECOND_NUMBER).await;

    // User enters second number
    bot.update(MockMessageText::new().text("4"));
    dispatch_and_check(&mut bot, &(text::YOUR_RESULT.to_owned() + "9")).await;
}

/// Test the complete subtraction flow from start to result
#[tokio::test]
async fn test_full_subtraction_flow() {
    let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
    bot.dependencies(deps![get_test_storage()]);

    // User sends /start
    dispatch_and_check(&mut bot, text::WHAT_DO_YOU_WANT).await;

    // User clicks "subtract" callback
    bot.update(MockCallbackQuery::new().data("subtract"));
    dispatch_and_check(&mut bot, text::ENTER_THE_FIRST_NUMBER).await;

    // User enters first number
    bot.update(MockMessageText::new().text("10"));
    dispatch_and_check(&mut bot, text::ENTER_THE_SECOND_NUMBER).await;

    // User enters second number
    bot.update(MockMessageText::new().text("3"));
    dispatch_and_check(&mut bot, &(text::YOUR_RESULT.to_owned() + "7")).await;
}

/// Test error handling when user sends invalid input (not a number)
#[tokio::test]
async fn test_invalid_number_input() {
    let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
    bot.dependencies(deps![get_test_storage()]);

    // User sends /start
    dispatch_and_check(&mut bot, text::WHAT_DO_YOU_WANT).await;

    // User clicks "add" callback
    bot.update(MockCallbackQuery::new().data("add"));
    dispatch_and_check(&mut bot, text::ENTER_THE_FIRST_NUMBER).await;

    // User enters invalid text instead of number
    bot.update(MockMessageText::new().text("not a number"));
    dispatch_and_check(&mut bot, text::PLEASE_ENTER_A_NUMBER).await;

    // User sends a photo instead of text
    bot.update(MockMessagePhoto::new());
    dispatch_and_check(&mut bot, text::PLEASE_SEND_TEXT).await;

    // User finally enters valid number and flow continues
    bot.update(MockMessageText::new().text("5"));
    dispatch_and_check(&mut bot, text::ENTER_THE_SECOND_NUMBER).await;
}

/// Test that multiple sequential calculations work (persistent server test)
#[tokio::test]
async fn test_multiple_calculations() {
    let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
    bot.dependencies(deps![get_test_storage()]);

    // First calculation: 2 + 3 = 5
    dispatch_and_check(&mut bot, text::WHAT_DO_YOU_WANT).await;
    bot.update(MockCallbackQuery::new().data("add"));
    dispatch_and_check(&mut bot, text::ENTER_THE_FIRST_NUMBER).await;
    bot.update(MockMessageText::new().text("2"));
    dispatch_and_check(&mut bot, text::ENTER_THE_SECOND_NUMBER).await;
    bot.update(MockMessageText::new().text("3"));
    dispatch_and_check(&mut bot, &(text::YOUR_RESULT.to_owned() + "5")).await;

    // Start another calculation: 10 - 4 = 6
    bot.update(MockMessageText::new().text("/start"));
    dispatch_and_check(&mut bot, text::WHAT_DO_YOU_WANT).await;
    bot.update(MockCallbackQuery::new().data("subtract"));
    dispatch_and_check(&mut bot, text::ENTER_THE_FIRST_NUMBER).await;
    bot.update(MockMessageText::new().text("10"));
    dispatch_and_check(&mut bot, text::ENTER_THE_SECOND_NUMBER).await;
    bot.update(MockMessageText::new().text("4"));
    dispatch_and_check(&mut bot, &(text::YOUR_RESULT.to_owned() + "6")).await;
}
