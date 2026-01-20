pub mod db;
pub mod handlers;
pub mod resources;
use std::error::Error;

use db::models::Phrase;
use dotenvy::dotenv;
use handlers::*;
use resources::{handler_tree::handler_tree, keyboards, text};
use teloxide::{
    dispatching::dialogue::{Dialogue, ErasedStorage, InMemStorage, Storage},
    prelude::*,
};

pub type MyDialogue = Dialogue<State, ErasedStorage<State>>;
pub type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;
pub type MyStorage = std::sync::Arc<ErasedStorage<State>>;

#[derive(Clone, PartialEq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum State {
    #[default]
    Start,
    ChangeNickname,
    WhatToDoWithPhrases,
    WhatIsNewPhraseEmoji,
    WhatIsNewPhraseText {
        emoji: String,
    },
    WhatIsNewPhraseBotText {
        emoji: String,
        text: String,
    },
    WhatPhraseToDelete {
        phrases: Vec<Phrase>,
    },
}

pub fn get_bot_storage() -> MyStorage {
    InMemStorage::<State>::new().erase()
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv().ok();

    let bot = Bot::from_env();

    // Create database pool
    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::create_pool(&database_url).await.expect("Failed to create database pool");
    db::run_migrations(&pool).await.expect("Failed to run migrations");

    Dispatcher::builder(bot, handler_tree())
        .dependencies(dptree::deps![get_bot_storage(), pool])
        .build()
        .dispatch()
        .await;
}
