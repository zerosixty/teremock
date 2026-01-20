use std::error::Error;

use dptree::{case, entry, filter};
use teloxide::{
    dispatching::{
        dialogue::{self, ErasedStorage},
        UpdateFilterExt, UpdateHandler,
    },
    prelude::*,
    types::Update,
};

use crate::{
    keyboards,
    private::{StartCommand, *},
    public::*,
    State,
};

/// Private chat handler tree with dialogue state management
fn private_branch() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    dialogue::enter::<Update, ErasedStorage<State>, State, _>()
        .branch(
            Update::filter_message()
                .filter_command::<StartCommand>()
                .branch(case![StartCommand::Start].endpoint(start))
                .branch(case![StartCommand::Cancel].endpoint(cancel)),
        )
        .branch(
            case![State::Start].branch(
                Update::filter_message()
                    .branch(
                        filter(|msg: Message| msg.text() == Some(keyboards::PROFILE_BUTTON))
                            .endpoint(profile),
                    )
                    .branch(
                        filter(|msg: Message| {
                            msg.text() == Some(keyboards::CHANGE_NICKNAME_BUTTON)
                        })
                        .endpoint(change_nickname),
                    )
                    .branch(
                        filter(|msg: Message| msg.text() == Some(keyboards::REMOVE_PHRASE_BUTTON))
                            .endpoint(delete_phrase),
                    )
                    .branch(
                        filter(|msg: Message| msg.text() == Some(keyboards::ADD_PHRASE_BUTTON))
                            .endpoint(add_phrase),
                    ),
            ),
        )
        .branch(
            case![State::ChangeNickname]
                .branch(Update::filter_message().endpoint(changed_nickname)),
        )
        .branch(
            case![State::WhatPhraseToDelete { phrases }]
                .branch(Update::filter_message().endpoint(deleted_phrase)),
        )
        .branch(
            entry()
                .branch(
                    case![State::WhatIsNewPhraseEmoji]
                        .branch(Update::filter_message().endpoint(what_is_new_phrase_text)),
                )
                .branch(
                    case![State::WhatIsNewPhraseText { emoji }]
                        .branch(Update::filter_message().endpoint(what_is_new_phrase_bot_text)),
                )
                .branch(
                    case![State::WhatIsNewPhraseBotText { emoji, text }]
                        .branch(Update::filter_message().endpoint(added_phrase)),
                ),
        )
}

/// Public chat handler tree
fn public_branch() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    Update::filter_message().endpoint(bot_phrase)
}

/// Handler tree containing all the bot logic.
pub fn handler_tree() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    entry()
        .branch(
            filter(|update: Update| update.chat().is_some() && update.chat().unwrap().is_private())
                .branch(private_branch()),
        )
        .branch(
            filter(|update: Update| {
                update.chat().is_some()
                    && (update.chat().unwrap().is_group() || update.chat().unwrap().is_supergroup())
            })
            .branch(public_branch()),
        )
}
