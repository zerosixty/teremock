use sqlx::PgPool;
use teloxide::{
    macros::BotCommands, payloads::SendMessageSetters, prelude::*, types::KeyboardRemove,
};

use crate::{
    db, db::models, keyboards, keyboards::menu_keyboard, text, HandlerResult, MyDialogue, State,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum StartCommand {
    #[command()]
    Start,
    Cancel,
}

//
//  Commands
//

pub async fn start(bot: Bot, msg: Message, dialogue: MyDialogue, pool: PgPool) -> HandlerResult {
    let user = db::get_user(&pool, msg.chat.id.0).await;
    if user.is_err() {
        db::create_user(&pool, msg.chat.id.0).await?;
    }
    bot.send_message(msg.chat.id, text::START)
        .reply_markup(keyboards::menu_keyboard())
        .await?;
    dialogue.update(State::Start).await?;
    Ok(())
}

pub async fn cancel(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    bot.send_message(msg.chat.id, text::CANCELED).await?;
    bot.send_message(msg.chat.id, text::MENU)
        .reply_markup(keyboards::menu_keyboard())
        .await?;
    dialogue.update(State::Start).await?;
    Ok(())
}

//
//   Menu buttons
//

async fn send_menu(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    bot.send_message(msg.chat.id, text::MENU)
        .reply_markup(menu_keyboard())
        .await?;
    dialogue.update(State::Start).await?;
    Ok(())
}

pub async fn profile(bot: Bot, msg: Message, pool: PgPool) -> HandlerResult {
    let user = db::get_user(&pool, msg.chat.id.0).await?;
    let all_phrases = db::get_user_phrases(&pool, msg.chat.id.0).await?;
    bot.send_message(msg.chat.id, text::profile(user.nickname, &all_phrases))
        .await?;
    Ok(())
}

pub async fn change_nickname(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    bot.send_message(msg.chat.id, text::CHANGE_NICKNAME)
        .reply_markup(KeyboardRemove::new())
        .await?;
    dialogue.update(State::ChangeNickname).await?;
    Ok(())
}

pub async fn delete_phrase(bot: Bot, msg: Message, dialogue: MyDialogue, pool: PgPool) -> HandlerResult {
    let user_phrases = db::get_user_phrases(&pool, msg.chat.id.0).await?;
    bot.send_message(msg.chat.id, text::delete_phrase(&user_phrases))
        .reply_markup(KeyboardRemove::new())
        .await?;
    dialogue
        .update(State::WhatPhraseToDelete {
            phrases: user_phrases,
        })
        .await?;
    Ok(())
}

pub async fn add_phrase(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    bot.send_message(msg.chat.id, text::what_is_new_phrase_emoji())
        .reply_markup(KeyboardRemove::new())
        .await?;
    dialogue.update(State::WhatIsNewPhraseEmoji).await?;
    Ok(())
}

//
//  Change nickname branch
//

pub async fn changed_nickname(bot: Bot, msg: Message, dialogue: MyDialogue, pool: PgPool) -> HandlerResult {
    let text = match msg.text() {
        Some(text) => text,
        None => {
            bot.send_message(msg.chat.id, text::PLEASE_SEND_TEXT)
                .await?;
            return Ok(());
        }
    };
    db::change_user_nickname(&pool, msg.chat.id.0, text.to_string()).await?;
    bot.send_message(msg.chat.id, text::CHANGED_NICKNAME.to_owned() + text)
        .await?;
    send_menu(bot, msg, dialogue).await
}

//
//   Delete phrase branch
//

pub async fn deleted_phrase(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    phrases: Vec<models::Phrase>,
    pool: PgPool,
) -> HandlerResult {
    let number = match msg.text() {
        Some(text) => match text.trim().parse::<usize>() {
            Ok(number) => number,
            Err(_) => {
                bot.send_message(msg.chat.id, text::PLEASE_SEND_NUMBER)
                    .await?;
                return Ok(());
            }
        },
        None => {
            bot.send_message(msg.chat.id, text::PLEASE_SEND_TEXT)
                .await?;
            return Ok(());
        }
    };
    if number > phrases.len() {
        bot.send_message(msg.chat.id, text::NO_SUCH_PHRASE).await?;
        return Ok(());
    }
    let phrase = &phrases[number - 1];
    db::delete_phrase(&pool, phrase.id).await?;
    bot.send_message(msg.chat.id, text::DELETED_PHRASE).await?;
    send_menu(bot, msg, dialogue).await
}

//
//  Add new phrase branch
//

pub async fn what_is_new_phrase_text(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
) -> HandlerResult {
    let text = match msg.text() {
        Some(text) => text,
        None => {
            bot.send_message(msg.chat.id, text::PLEASE_SEND_TEXT)
                .await?;
            return Ok(());
        }
    };
    if text.chars().count() > 3 {
        bot.send_message(msg.chat.id, text::NO_MORE_CHARACTERS)
            .await?;
        return Ok(());
    }
    bot.send_message(msg.chat.id, text::what_is_new_phrase_text(text))
        .await?;
    dialogue
        .update(State::WhatIsNewPhraseText {
            emoji: text.to_string(),
        })
        .await?;
    Ok(())
}

pub async fn what_is_new_phrase_bot_text(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    emoji: String,
) -> HandlerResult {
    let text = match msg.text() {
        Some(text) => text,
        None => {
            bot.send_message(msg.chat.id, text::PLEASE_SEND_TEXT)
                .await?;
            return Ok(());
        }
    };
    bot.send_message(msg.chat.id, text::what_is_new_phrase_bot_text(&emoji, text))
        .await?;
    dialogue
        .update(State::WhatIsNewPhraseBotText {
            emoji,
            text: text.to_string(),
        })
        .await?;
    Ok(())
}

pub async fn added_phrase(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    state_data: (String, String),
    pool: PgPool,
) -> HandlerResult {
    let text = match msg.text() {
        Some(text) => text,
        None => {
            bot.send_message(msg.chat.id, text::PLEASE_SEND_TEXT)
                .await?;
            return Ok(());
        }
    };
    bot.send_message(
        msg.chat.id,
        text::added_phrase(&state_data.0, &state_data.1, text),
    )
    .await?;
    db::create_phrase(&pool, msg.chat.id.0, state_data.0, state_data.1, text.to_string()).await?;
    send_menu(bot, msg, dialogue).await
}

//
//   Tests
//

#[cfg(test)]
mod tests {
    use sqlx::PgPool;
    use teloxide::{
        dispatching::dialogue::{InMemStorage, Storage},
        dptree::deps,
        types::ReplyMarkup,
    };
    use teremock::{MockBot, MockMessageDocument, MockMessageText, MockUser};

    use super::*;
    use crate::{resources::handler_tree::handler_tree, MyStorage, State};

    /// Creates an in-memory storage for tests (no Redis required)
    fn get_test_storage() -> MyStorage {
        InMemStorage::<State>::new().erase()
    }

    /// Test /start command creates user and shows menu
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn test_start(pool: PgPool) {
        let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;

        bot.dependencies(deps![get_test_storage(), pool.clone()]);

        bot.dispatch().await;
        let responses = bot.get_responses();

        assert_eq!(
            responses.sent_messages.last().unwrap().text(),
            Some(text::START)
        );
        assert_eq!(
            responses
                .sent_messages_text
                .last()
                .unwrap()
                .bot_request
                .reply_markup,
            Some(ReplyMarkup::Keyboard(keyboards::menu_keyboard()))
        );
        assert_eq!(db::get_user(&pool, MockUser::ID as i64).await.unwrap().nickname, None);
    }

    /// Test full cancel flow: start -> change nickname -> cancel
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn test_cancel_flow(pool: PgPool) {
        let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
        bot.dependencies(deps![get_test_storage(), pool.clone()]);

        // Start the bot
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::START)
        );

        // Click change nickname button
        bot.update(MockMessageText::new().text(keyboards::CHANGE_NICKNAME_BUTTON));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::CHANGE_NICKNAME)
        );

        // Cancel the operation
        bot.update(MockMessageText::new().text("/cancel"));
        bot.dispatch().await;

        let responses = bot.get_responses();
        assert_eq!(
            responses.sent_messages.first().unwrap().text(),
            Some(text::CANCELED)
        );
        assert_eq!(
            responses
                .sent_messages_text
                .last()
                .unwrap()
                .bot_request
                .reply_markup,
            Some(ReplyMarkup::Keyboard(keyboards::menu_keyboard()))
        );
    }

    /// Test profile button shows user profile
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn test_profile(pool: PgPool) {
        db::full_user_redeletion(&pool, MockUser::ID as i64, None).await.unwrap();
        db::create_phrase(
            &pool,
            MockUser::ID as i64,
            "🤗".to_string(),
            "hug".to_string(),
            "(me) hugged (reply)".to_string(),
        )
        .await
        .unwrap();

        let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
        bot.dependencies(deps![get_test_storage(), pool.clone()]);

        // Start the bot
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::START)
        );

        // Click profile button
        bot.update(MockMessageText::new().text(keyboards::PROFILE_BUTTON));
        bot.dispatch().await;

        let user = db::get_user(&pool, MockUser::ID as i64).await.unwrap();
        let all_phrases = db::get_user_phrases(&pool, MockUser::ID as i64).await.unwrap();
        let responses = bot.get_responses();
        let expected = text::profile(user.nickname, &all_phrases);
        assert_eq!(
            responses.sent_messages.last().unwrap().text(),
            Some(expected.as_str())
        );
    }

    /// Test full change nickname flow
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn test_change_nickname_flow(pool: PgPool) {
        db::full_user_redeletion(&pool, MockUser::ID as i64, None).await.unwrap();

        let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
        bot.dependencies(deps![get_test_storage(), pool.clone()]);

        // Start the bot
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::START)
        );

        // Click change nickname button
        bot.update(MockMessageText::new().text(keyboards::CHANGE_NICKNAME_BUTTON));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::CHANGE_NICKNAME)
        );

        // Enter new nickname
        bot.update(MockMessageText::new().text("nickname"));
        bot.dispatch().await;

        let user = db::get_user(&pool, MockUser::ID as i64).await.unwrap();
        let responses = bot.get_responses();
        assert_eq!(
            responses.sent_messages.first().unwrap().text(),
            Some(text::CHANGED_NICKNAME.to_owned() + "nickname").as_deref()
        );
        assert_eq!(user.nickname, Some("nickname".to_string()));
    }

    /// Test full delete phrase flow with error handling
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn test_delete_phrase_flow(pool: PgPool) {
        db::full_user_redeletion(&pool, MockUser::ID as i64, None).await.unwrap();
        db::create_phrase(
            &pool,
            MockUser::ID as i64,
            "🤗".to_string(),
            "hug".to_string(),
            "(me) hugged (reply)".to_string(),
        )
        .await
        .unwrap();
        let all_phrases = db::get_user_phrases(&pool, MockUser::ID as i64).await.unwrap();

        let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
        bot.dependencies(deps![get_test_storage(), pool.clone()]);

        // Start the bot
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::START)
        );

        // Click remove phrase button
        bot.update(MockMessageText::new().text(keyboards::REMOVE_PHRASE_BUTTON));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::delete_phrase(&all_phrases).as_str())
        );

        // Try sending not a number
        bot.update(MockMessageText::new().text("not a number"));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::PLEASE_SEND_NUMBER)
        );

        // Try sending a document
        bot.update(MockMessageDocument::new());
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::PLEASE_SEND_TEXT)
        );

        // Try sending wrong number
        bot.update(MockMessageText::new().text("100"));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::NO_SUCH_PHRASE)
        );

        // Send correct number
        bot.update(MockMessageText::new().text("1"));
        bot.dispatch().await;

        let new_all_phrases = db::get_user_phrases(&pool, MockUser::ID as i64).await.unwrap();
        let responses = bot.get_responses();
        assert_eq!(
            responses.sent_messages.first().unwrap().text(),
            Some(text::DELETED_PHRASE)
        );

        assert_eq!(all_phrases.len() - 1, new_all_phrases.len());
    }

    /// Test full add phrase flow with error handling
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn test_add_phrase_flow(pool: PgPool) {
        db::full_user_redeletion(&pool, MockUser::ID as i64, None).await.unwrap();

        let mut bot = MockBot::new(MockMessageText::new().text("/start"), handler_tree()).await;
        bot.dependencies(deps![get_test_storage(), pool.clone()]);

        // Start the bot
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::START)
        );

        // Click add phrase button
        bot.update(MockMessageText::new().text(keyboards::ADD_PHRASE_BUTTON));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::what_is_new_phrase_emoji().as_str())
        );

        // Try entering too many characters
        bot.update(MockMessageText::new().text("🤗🤗🤗🤗"));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::NO_MORE_CHARACTERS)
        );

        // Enter valid emoji
        bot.update(MockMessageText::new().text("🤗"));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::what_is_new_phrase_text("🤗").as_str())
        );

        // Enter phrase text
        bot.update(MockMessageText::new().text("hug"));
        bot.dispatch().await;
        assert_eq!(
            bot.get_responses().sent_messages.last().unwrap().text(),
            Some(text::what_is_new_phrase_bot_text("🤗", "hug").as_str())
        );

        // Enter bot text
        bot.update(MockMessageText::new().text("(me) hugged (reply)"));
        bot.dispatch().await;

        let responses = bot.get_responses();
        assert_eq!(
            responses.sent_messages.first().unwrap().text(),
            Some(text::added_phrase("🤗", "hug", "(me) hugged (reply)")).as_deref()
        );
        assert_eq!(
            db::get_user_phrases(&pool, MockUser::ID as i64)
                .await
                .unwrap()
                .first()
                .unwrap()
                .text,
            "hug".to_string()
        );
    }
}
