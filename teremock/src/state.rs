use teloxide::{
    prelude::*,
    types::{File, FileMeta, MessageId, MessageKind},
};

use crate::{server::messages::Messages, MockMessageText, Responses};

/// Extract file metadata directly from message fields without JSON serialization.
/// This is more efficient than serializing the entire message to JSON.
fn extract_file_meta(message: &Message) -> Option<FileMeta> {
    // Check all possible media types that contain files
    if let Some(doc) = message.document() {
        return Some(doc.file.clone());
    }
    if let Some(photo) = message.photo() {
        // Photos are sorted by size, get the largest one
        return photo.last().map(|p| p.file.clone());
    }
    if let Some(audio) = message.audio() {
        return Some(audio.file.clone());
    }
    if let Some(video) = message.video() {
        return Some(video.file.clone());
    }
    if let Some(voice) = message.voice() {
        return Some(voice.file.clone());
    }
    if let Some(video_note) = message.video_note() {
        return Some(video_note.file.clone());
    }
    if let Some(animation) = message.animation() {
        return Some(animation.file.clone());
    }
    if let Some(sticker) = message.sticker() {
        return Some(sticker.file.clone());
    }
    None
}

/// Default file path used when storing files from messages.
/// The actual path doesn't matter for testing purposes.
const DEFAULT_FILE_PATH: &str = "some_path.txt";

#[derive(Default)]
pub(crate) struct State {
    pub files: Vec<File>,
    pub responses: Responses,
    pub messages: Messages,
}

impl State {
    pub fn reset(&mut self) {
        self.responses = Responses::default();
    }

    pub(crate) fn add_message(&mut self, message: &mut Message) {
        let max_id = self.messages.max_message_id();
        let maybe_message = self.messages.get_message(message.id.0);

        // If message exists in the database, and it isn't a default,
        // let it be, the user knows best
        if maybe_message.is_some() && message.id != MessageId(MockMessageText::ID) {
            log::debug!(
                "Not inserting message with id {}, this id exists in the database.",
                message.id
            );
            return;
        }

        if message.id.0 <= max_id || maybe_message.is_some() {
            message.id = MessageId(max_id + 1);
        }

        // Extract file metadata directly without JSON serialization
        if let Some(file_meta) = extract_file_meta(message) {
            let file = File {
                meta: file_meta,
                path: DEFAULT_FILE_PATH.to_string(),
            };
            self.files.push(file);
        }

        if let MessageKind::Common(ref mut message_kind) = message.kind {
            if let Some(ref mut reply_message) = message_kind.reply_to_message {
                self.add_message(reply_message);
            }
        }
        log::debug!("Inserted message with {}.", message.id);
        self.messages.add_message(message.clone());
    }

    pub(crate) fn edit_message(&mut self, message: &mut Message) {
        let old_message = self.messages.get_message(message.id.0);

        if old_message.is_none() {
            log::error!(
                "Not editing message with id {}, this id does not exist in the database.",
                message.id
            );
            return;
        }

        // Extract file metadata directly without JSON serialization
        if let Some(file_meta) = extract_file_meta(message) {
            // Only add if this file doesn't already exist
            if self
                .files
                .iter()
                .all(|f| f.meta.unique_id != file_meta.unique_id)
            {
                let file = File {
                    meta: file_meta,
                    path: DEFAULT_FILE_PATH.to_string(),
                };
                self.files.push(file);
            }
        }

        log::debug!("Edited message with {}.", message.id);
        self.messages.edit_message(message.clone());
    }
}
