use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, FromRow, Serialize, Deserialize)]
pub struct Phrase {
    pub id: i32,
    pub user_id: i64,
    pub emoji: String,
    pub text: String,
    pub bot_text: String,
}
