//! A fake telegram bot API for testing purposes. Read more in teremock crate.
mod routes;
use std::{
    error::Error,
    io,
    net::TcpListener,
    sync::{Arc, Mutex},
};

use actix_web::{
    web::{self, get, post, scope, Data, ServiceConfig},
    App, HttpResponse, HttpServer, Responder,
};
pub use responses::*;
use routes::{
    answer_callback_query::*, ban_chat_member::*, copy_message::*, delete_message::*,
    delete_messages::*, download_file::download_file, edit_message_caption::*,
    edit_message_reply_markup::*, edit_message_text::*, forward_message::*, get_file::*, get_me::*,
    get_updates::*, get_webhook_info::*, pin_chat_message::*, restrict_chat_member::*,
    send_animation::*, send_audio::*, send_chat_action::*, send_contact::*, send_dice::*,
    send_document::*, send_invoice::*, send_location::*, send_media_group::*, send_message::*,
    send_photo::*, send_poll::*, send_sticker::*, send_venue::*, send_video::*, send_video_note::*,
    send_voice::*, set_message_reaction::*, set_my_commands::*, unban_chat_member::*,
    unpin_all_chat_messages::*, unpin_chat_message::*,
};
pub use routes::{
    copy_message::CopyMessageBody, delete_message::DeleteMessageBody,
    delete_messages::DeleteMessagesBody, edit_message_caption::EditMessageCaptionBody,
    edit_message_reply_markup::EditMessageReplyMarkupBody, edit_message_text::EditMessageTextBody,
    forward_message::ForwardMessageBody, send_animation::SendMessageAnimationBody,
    send_audio::SendMessageAudioBody, send_contact::SendMessageContactBody,
    send_dice::SendMessageDiceBody, send_document::SendMessageDocumentBody,
    send_invoice::SendMessageInvoiceBody, send_location::SendMessageLocationBody,
    send_media_group::SendMediaGroupBody, send_message::SendMessageTextBody,
    send_photo::SendMessagePhotoBody, send_poll::SendMessagePollBody,
    send_sticker::SendMessageStickerBody, send_venue::SendMessageVenueBody,
    send_video::SendMessageVideoBody, send_video_note::SendMessageVideoNoteBody,
};
use teloxide::types::Me;
use tokio::{
    sync::mpsc::{channel, Sender},
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::CancellationToken;

use crate::state::State;

pub mod messages;
pub mod responses;

pub(crate) struct ServerManager {
    pub port: u16,
    /// The server task handle. Kept for potential future use and for the Drop impl.
    #[allow(dead_code)]
    server: Option<JoinHandle<()>>,
    cancel_token: CancellationToken,
}

impl Drop for ServerManager {
    fn drop(&mut self) {
        // Trigger graceful shutdown by cancelling the token.
        // The shutdown task spawned in run_server() will handle stopping the actix server.
        self.cancel_token.cancel();
        // Note: We cannot await the server handle here since Drop is synchronous.
        // The tokio runtime will clean up the spawned task when it completes.
    }
}

#[warn(clippy::unwrap_used)]
impl ServerManager {
    pub(crate) async fn start(me: Me, state: Arc<Mutex<State>>) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        let cancel_token = CancellationToken::new();
        let (tx, mut rx) = channel::<()>(100);

        let server = tokio::spawn(run_server(
            listener,
            me,
            state.clone(),
            cancel_token.clone(),
            tx,
        ));
        // Waits until the server is ready
        rx.recv().await;

        Ok(Self {
            port,
            cancel_token,
            server: Some(server),
        })
    }

    /// Gracefully stops the server and waits for it to complete.
    ///
    /// This is an async alternative to the `Drop` implementation when you need
    /// to ensure the server has fully stopped before continuing.
    ///
    /// Returns `None` if the server was already stopped.
    #[allow(dead_code)]
    pub(crate) async fn stop(&mut self) -> Option<Result<(), JoinError>> {
        self.cancel_token.cancel();
        if let Some(server) = self.server.take() {
            Some(server.await)
        } else {
            None
        }
    }
}

async fn run_server(
    listener: TcpListener,
    me: Me,
    state: Arc<Mutex<State>>,
    cancel_token: CancellationToken,
    tx: Sender<()>,
) {
    let server = match create_server(listener, me, state) {
        Ok(server) => server,
        Err(e) => {
            log::error!("Failed to create mock server: {}", e);
            return;
        }
    };

    if let Err(e) = tx.send(()).await {
        log::error!("Failed to send server ready signal: {}", e);
        return;
    }

    let server_handle = server.handle();

    tokio::spawn(async move {
        cancel_token.cancelled().await;
        server_handle.stop(false).await;
    });

    if let Err(e) = server.await {
        log::error!("Mock server error: {}", e);
    }
}

fn create_server(
    listener: TcpListener,
    me: Me,
    state: Arc<Mutex<State>>,
) -> io::Result<actix_web::dev::Server> {
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(Data::new(me.clone()))
            .app_data(Data::from(state.clone()))
            .configure(set_routes)
    })
    // Use single worker for test server - no need for multiple workers
    .workers(1)
    .listen(listener)?
    .run())
}

fn set_routes(cfg: &mut ServiceConfig) {
    cfg.route("/file/bot{token}/{file_name}", get().to(download_file))
        .service(scope("/bot{token}").configure(set_bot_routes));
}

fn set_bot_routes(cfg: &mut ServiceConfig) {
    cfg.route("/GetFile", post().to(get_file))
        .route("/SendMessage", post().to(send_message))
        .route("/GetWebhookInfo", post().to(get_webhook_info))
        .route("/GetMe", post().to(get_me))
        .route("/GetUpdates", post().to(get_updates))
        .route("/SendPhoto", post().to(send_photo))
        .route("/SendVideo", post().to(send_video))
        .route("/SendVoice", post().to(send_voice))
        .route("/SendAudio", post().to(send_audio))
        .route("/SendVideoNote", post().to(send_video_note))
        .route("/SendDocument", post().to(send_document))
        .route("/SendAnimation", post().to(send_animation))
        .route("/SendLocation", post().to(send_location))
        .route("/SendVenue", post().to(send_venue))
        .route("/SendContact", post().to(send_contact))
        .route("/SendSticker", post().to(send_sticker))
        .route("/SendChatAction", post().to(send_chat_action))
        .route("/SendDice", post().to(send_dice))
        .route("/SendPoll", post().to(send_poll))
        .route("/SendMediaGroup", post().to(send_media_group))
        .route("/SendInvoice", post().to(send_invoice))
        .route("/EditMessageText", post().to(edit_message_text))
        .route("/EditMessageCaption", post().to(edit_message_caption))
        .route(
            "/EditMessageReplyMarkup",
            post().to(edit_message_reply_markup),
        )
        .route("/DeleteMessage", post().to(delete_message))
        .route("/DeleteMessages", post().to(delete_messages))
        .route("/ForwardMessage", post().to(forward_message))
        .route("/CopyMessage", post().to(copy_message))
        .route("/AnswerCallbackQuery", post().to(answer_callback_query))
        .route("/PinChatMessage", post().to(pin_chat_message))
        .route("/UnpinChatMessage", post().to(unpin_chat_message))
        .route("/UnpinAllChatMessages", post().to(unpin_all_chat_messages))
        .route("/BanChatMember", post().to(ban_chat_member))
        .route("/UnbanChatMember", post().to(unban_chat_member))
        .route("/RestrictChatMember", post().to(restrict_chat_member))
        .route("/SetMessageReaction", post().to(set_message_reaction))
        .route("/SetMyCommands", post().to(set_my_commands))
        .route("/{unknown_endpoint}", post().to(unknown_endpoint));
}

async fn unknown_endpoint(path: web::Path<(String, String)>) -> impl Responder {
    HttpResponse::InternalServerError().message_body(format!("Endpoint \"{}\" is not yet implemented! Please make an issue to https://github.com/LasterAlex/teloxide_tests/issues/new?assignees=&labels=no+endpoint&projects=&template=add-endpoint-template.md&title=", path.1))
}
