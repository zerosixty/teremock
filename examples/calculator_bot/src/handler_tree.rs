use std::error::Error;

use dptree::case;
use teloxide::{
    dispatching::{
        dialogue::{self, ErasedStorage},
        UpdateFilterExt, UpdateHandler,
    },
    prelude::*,
    types::Update,
};

use crate::{
    handlers::{StartCommand, *},
    State,
};

/// Handler tree containing all the bot logic.
pub fn handler_tree() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    dialogue::enter::<Update, ErasedStorage<State>, State, _>()
        .branch(
            Update::filter_message()
                .filter_command::<StartCommand>()
                .branch(case![StartCommand::Start].endpoint(start)),
        )
        .branch(
            Update::filter_callback_query()
                .branch(case![State::WhatDoYouWant].endpoint(what_is_the_first_number)),
        )
        .branch(
            Update::filter_message()
                .branch(
                    case![State::GetFirstNumber { operation }].endpoint(what_is_the_second_number),
                )
                .branch(
                    case![State::GetSecondNumber {
                        first_number,
                        operation
                    }]
                    .endpoint(get_result),
                ),
        )
}
