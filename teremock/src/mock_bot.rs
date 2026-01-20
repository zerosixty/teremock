//! Mock bot that sends requests to the fake server with persistent server architecture
//!
//! Key differences from teloxide_tests:
//! - Server starts once in `new()` and is reused across all `dispatch()` calls
//! - Each dispatch runs in a separate tokio task to prevent stack overflow
//! - No dialogue state manipulation (black-box testing philosophy)
//! - Works with default 2MB stack (no custom thread builder needed)
use std::{
    fmt::Debug,
    hash::Hash,
    sync::{atomic::AtomicI32, Arc, Mutex},
};

use teloxide::{
    dispatching::UpdateHandler,
    error_handlers::ErrorHandler,
    prelude::*,
    stop::mk_stop_token,
    types::{MaybeInaccessibleMessage, Me, UpdateKind},
};

pub use crate::utils::DistributionKey;
use crate::{
    dataset::{IntoUpdate, MockMe},
    server,
    server::ServerManager,
    state::State,
    utils::default_distribution_function,
};

/// A mocked bot that sends requests to the fake server.
///
/// This implementation features:
/// - **Persistent server**: Server starts once in `new()` and is reused across all dispatches
/// - **Stack overflow prevention**: Each dispatch runs in a separate tokio task
/// - **Black-box testing**: No dialogue state manipulation API
///
/// # Example
///
/// ```no_run
/// use teloxide::dispatching::UpdateHandler;
/// use teloxide::types::Update;
/// use teremock::{MockBot, MockMessageText};
/// use teloxide::dispatching::dialogue::GetChatId;
/// use teloxide::prelude::*;
///
/// fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
///     teloxide::dptree::entry().endpoint(|update: Update, bot: Bot| async move {
///         bot.send_message(update.chat_id().unwrap(), "Hello!").await?;
///         Ok(())
///     })
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mut bot = MockBot::new(MockMessageText::new().text("Hi!"), handler_tree()).await;
///     bot.dispatch().await;
///     let responses = bot.get_responses();
///     let message = responses
///         .sent_messages
///         .last()
///         .expect("No sent messages were detected!");
///     assert_eq!(message.text(), Some("Hello!"));
/// }
/// ```
pub struct MockBot<Err, Key> {
    /// The bot with a fake server url
    pub bot: Bot,
    /// The handler tree wrapped in Arc for efficient cloning across tasks
    handler_tree: Arc<UpdateHandler<Err>>,
    /// Updates to send as user
    pub updates: Vec<Update>,
    /// Bot parameters are here
    pub me: Me,
    /// If you have something like a state, you should add the storage here using .dependencies()
    pub dependencies: DependencyMap,

    distribution_f: fn(&Update) -> Option<Key>,
    error_handler: Arc<dyn ErrorHandler<Err> + Send + Sync>,

    current_update_id: AtomicI32,
    state: Arc<Mutex<State>>,
    /// Persistent server instance - started once, reused across all dispatches.
    /// When MockBot is dropped, the server's Drop impl triggers graceful shutdown.
    /// This field is never explicitly read, but is kept alive for its Drop impl.
    #[allow(dead_code)]
    server: ServerManager,
    /// The API URL for the bot
    api_url: url::Url,
}

impl<Err> MockBot<Err, DistributionKey>
where
    Err: Debug + Send + Sync + 'static,
{
    /// Creates a new MockBot with a persistent server.
    ///
    /// The server is started immediately and will be reused for all subsequent
    /// `dispatch()` calls. This is much faster than the original teloxide_tests
    /// which restarts the server on every dispatch.
    ///
    /// Note: This is an async function because it starts the server.
    ///
    /// # Arguments
    ///
    /// * `update` - Any Mock type that can be turned into Updates (MockMessageText,
    ///   MockCallbackQuery, Vec<MockMessagePhoto>, etc.)
    /// * `handler_tree` - The dptree handler tree (same as in teloxide dispatching)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use teloxide::dispatching::UpdateHandler;
    /// use teremock::{MockBot, MockMessageText};
    ///
    /// fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    ///     teloxide::dptree::entry() /* your handlers go here */
    /// }
    ///
    /// #[tokio::test]
    /// async fn test_example() {
    ///     let mut bot = MockBot::new(MockMessageText::new().text("Hi!"), handler_tree()).await;
    ///     bot.dispatch().await;
    /// }
    /// ```
    pub async fn new<T>(update: T, handler_tree: UpdateHandler<Err>) -> Self
    where
        T: IntoUpdate,
        Err: Debug,
    {
        let _ = pretty_env_logger::try_init();

        let token = "1234567890:QWERTYUIOPASDFGHJKLZXCVBNMQWERTYUIO";
        let bot = Bot::new(token);
        let current_update_id = AtomicI32::new(42);
        let state = Arc::new(Mutex::new(State::default()));
        let me = MockMe::new().build();

        // Start the server immediately - it will be reused for all dispatches
        let server = ServerManager::start(me.clone(), state.clone())
            .await
            .expect("Failed to start mock server");

        let api_url = url::Url::parse(&format!("http://127.0.0.1:{}", server.port))
            .expect("Failed to parse API URL");

        Self {
            bot,
            me,
            updates: update.into_update(&current_update_id),
            handler_tree: Arc::new(handler_tree), // Wrap in Arc for efficient cloning
            dependencies: DependencyMap::new(),
            error_handler: LoggingErrorHandler::new(),
            distribution_f: default_distribution_function,
            current_update_id,
            state,
            server,
            api_url,
        }
    }
}

impl<Err, Key> MockBot<Err, Key>
where
    Err: Debug + Send + Sync + 'static,
    Key: Hash + Eq + Clone + Send + 'static,
{
    /// Same as [`new`], but it inserts a distribution_function into the dispatcher
    ///
    /// [`new`]: crate::MockBot::new
    pub async fn new_with_distribution_function<T>(
        update: T,
        handler_tree: UpdateHandler<Err>,
        f: fn(&Update) -> Option<Key>,
    ) -> Self
    where
        T: IntoUpdate,
        Err: Debug,
    {
        let _ = pretty_env_logger::try_init();

        let token = "1234567890:QWERTYUIOPASDFGHJKLZXCVBNMQWERTYUIO";
        let bot = Bot::new(token);
        let current_update_id = AtomicI32::new(42);
        let state = Arc::new(Mutex::new(State::default()));
        let me = MockMe::new().build();

        let server = ServerManager::start(me.clone(), state.clone())
            .await
            .expect("Failed to start mock server");

        let api_url = url::Url::parse(&format!("http://127.0.0.1:{}", server.port))
            .expect("Failed to parse API URL");

        Self {
            bot,
            me,
            updates: update.into_update(&current_update_id),
            handler_tree: Arc::new(handler_tree),
            dependencies: DependencyMap::new(),
            error_handler: LoggingErrorHandler::new(),
            distribution_f: f,
            current_update_id,
            state,
            server,
            api_url,
        }
    }

    /// Sets the dependencies of the dptree. The same as deps![] in bot dispatching.
    ///
    /// Use this to add dependencies like database pools, storage, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use dptree::deps;
    /// use teloxide::dispatching::dialogue::InMemStorage;
    ///
    /// bot.dependencies(deps![InMemStorage::<State>::new()]);
    /// ```
    pub fn dependencies(&mut self, deps: DependencyMap) {
        self.dependencies = deps;
    }

    /// Sets the bot parameters (like supports_inline_queries, first_name, etc.)
    pub fn me(&mut self, me: MockMe) {
        self.me = me.build();
    }

    /// Sets the updates. Useful for reusing the same mocked bot instance.
    ///
    /// You can pass in `vec![MockMessagePhoto]` or any other IntoUpdate type!
    pub fn update<T: IntoUpdate>(&mut self, update: T) {
        self.updates = update.into_update(&self.current_update_id);
    }

    /// Sets the error_handler for the Dispatcher
    pub fn error_handler(&mut self, handler: Arc<dyn ErrorHandler<Err> + Send + Sync>) {
        self.error_handler = handler;
    }

    /// Returns the API URL that the bot is using
    pub fn api_url(&self) -> &url::Url {
        &self.api_url
    }

    /// Just inserts the updates into the state, returning them
    fn insert_updates(&self, updates: &mut [Update]) {
        let mut state = self.state.lock().unwrap();
        for update in updates.iter_mut() {
            match &mut update.kind {
                UpdateKind::Message(ref mut message) => {
                    state.add_message(message);
                }
                UpdateKind::EditedMessage(ref mut message) => {
                    state.edit_message(message);
                }
                UpdateKind::CallbackQuery(ref mut callback) => {
                    if let Some(MaybeInaccessibleMessage::Regular(ref mut message)) =
                        callback.message
                    {
                        state.add_message(message);
                    }
                }
                _ => {}
            }
        }
    }

    /// Actually dispatches the bot, calling the update through the handler tree.
    ///
    /// All the requests made through the bot will be stored in `responses`, and can be retrieved
    /// with `get_responses`. All the responses are unique to that dispatch, and will be erased for
    /// every new dispatch.
    ///
    /// # Stack overflow protection
    ///
    /// Each dispatch runs in a separate tokio task to prevent stack overflow
    /// when performing many sequential dispatches (50+).
    pub async fn dispatch(&mut self) {
        // Clear previous responses but keep server alive
        self.state.lock().unwrap().reset();

        let mut updates = self.updates.clone();
        self.insert_updates(&mut updates);

        // Clone bot and set API URL - bot.clone() is cheap (just Arc clones internally)
        let bot = self.bot.clone().set_api_url(self.api_url.clone());

        // Clone what we need for the spawned task
        let handler_tree = Arc::clone(&self.handler_tree);
        let deps = self.dependencies.clone();
        let distribution_f = self.distribution_f;
        let error_handler = self.error_handler.clone();

        // Spawn dispatch in separate tokio task to prevent stack overflow
        // across many sequential dispatches
        let handle = tokio::task::spawn(async move {
            Dispatcher::builder(bot, (*handler_tree).clone())
                .dependencies(deps)
                .distribution_function(distribution_f)
                .error_handler(error_handler)
                .build()
                .dispatch_with_listener(
                    SingleUpdateListener::new(updates),
                    LoggingErrorHandler::new(),
                )
                .await;
        });

        handle.await.expect("Dispatch task panicked!");
    }

    /// Returns the responses stored in `responses`
    pub fn get_responses(&self) -> server::Responses {
        self.state.lock().unwrap().responses.clone()
    }
}

/// A simple update listener that processes updates and stops.
struct SingleUpdateListener {
    updates: Vec<Update>,
}

impl SingleUpdateListener {
    fn new(updates: Vec<Update>) -> Self {
        Self { updates }
    }
}

impl teloxide::update_listeners::UpdateListener for SingleUpdateListener {
    type Err = std::convert::Infallible;

    fn stop_token(&mut self) -> teloxide::stop::StopToken {
        // Create a lightweight stop token without constructing a full Polling listener
        let (token, _flag) = mk_stop_token();
        token
    }
}

impl<'a> teloxide::update_listeners::AsUpdateStream<'a> for SingleUpdateListener {
    type StreamErr = std::convert::Infallible;
    type Stream = SingleUpdateStream;

    fn as_stream(&'a mut self) -> Self::Stream {
        SingleUpdateStream {
            updates: std::mem::take(&mut self.updates).into(),
        }
    }
}

struct SingleUpdateStream {
    updates: std::collections::VecDeque<Update>,
}

impl futures_util::Stream for SingleUpdateStream {
    type Item = Result<Update, std::convert::Infallible>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.updates.pop_front() {
            Some(update) => std::task::Poll::Ready(Some(Ok(update))),
            None => std::task::Poll::Ready(None),
        }
    }
}
