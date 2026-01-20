use std::sync::atomic::Ordering;

use chrono::{DateTime, Utc};
use teloxide::types::{
    MessageEntity, Poll, PollId, PollOption, PollType, Seconds, Update, UpdateId, UpdateKind,
};
use teremock_macros::Changeable;

use super::{IntoUpdate, MockMessagePoll};

#[derive(Changeable, Clone)]
pub struct MockUpdatePoll {
    pub poll_id: PollId,
    pub question: String,
    pub question_entities: Option<Vec<MessageEntity>>,
    pub options: Vec<PollOption>,
    pub is_closed: bool,
    pub total_voter_count: u32,
    pub is_anonymous: bool,
    pub poll_type: PollType,
    pub allows_multiple_answers: bool,
    pub correct_option_id: Option<u8>,
    pub explanation: Option<String>,
    pub explanation_entities: Option<Vec<MessageEntity>>,
    pub open_period: Option<Seconds>,
    pub close_date: Option<DateTime<Utc>>,
}

impl MockUpdatePoll {
    /// Creates a new easily changable poll update builder
    ///
    /// # Example
    /// ```
    /// use teloxide::types::PollId;
    /// let update = teremock::MockUpdatePoll::new()
    ///     .poll_id(PollId::from("123456"));
    ///
    /// assert_eq!(update.poll_id, PollId::from("123456"));
    /// ```
    pub fn new() -> Self {
        let poll = MockMessagePoll::new();
        Self {
            poll_id: poll.poll_id,
            question: poll.question,
            question_entities: poll.question_entities,
            options: poll.options,
            is_closed: poll.is_closed,
            total_voter_count: poll.total_voter_count,
            is_anonymous: poll.is_anonymous,
            poll_type: poll.poll_type,
            allows_multiple_answers: poll.allows_multiple_answers,
            correct_option_id: poll.correct_option_id,
            explanation: poll.explanation,
            explanation_entities: poll.explanation_entities,
            open_period: poll.open_period,
            close_date: poll.close_date,
        }
    }
}

impl IntoUpdate for MockUpdatePoll {
    fn into_update(self, id: &std::sync::atomic::AtomicI32) -> Vec<Update> {
        vec![Update {
            id: UpdateId(id.fetch_add(1, Ordering::Relaxed) as u32),
            kind: UpdateKind::Poll(Poll {
                id: self.poll_id,
                question: self.question,
                question_entities: self.question_entities,
                options: self.options,
                is_closed: self.is_closed,
                total_voter_count: self.total_voter_count,
                is_anonymous: self.is_anonymous,
                poll_type: self.poll_type,
                allows_multiple_answers: self.allows_multiple_answers,
                correct_option_id: self.correct_option_id,
                explanation: self.explanation,
                explanation_entities: self.explanation_entities,
                open_period: self.open_period,
                close_date: self.close_date,
            }),
        }]
    }
}

// From implementation for ergonomic API - allows passing mock builders directly without .build()

impl From<MockUpdatePoll> for Poll {
    fn from(mock: MockUpdatePoll) -> Self {
        Poll {
            id: mock.poll_id,
            question: mock.question,
            question_entities: mock.question_entities,
            options: mock.options,
            is_closed: mock.is_closed,
            total_voter_count: mock.total_voter_count,
            is_anonymous: mock.is_anonymous,
            poll_type: mock.poll_type,
            allows_multiple_answers: mock.allows_multiple_answers,
            correct_option_id: mock.correct_option_id,
            explanation: mock.explanation,
            explanation_entities: mock.explanation_entities,
            open_period: mock.open_period,
            close_date: mock.close_date,
        }
    }
}
