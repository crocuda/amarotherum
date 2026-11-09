mod servo_state;
pub use servo_state::Servo;

// Winit
use iced::Event;
use iced::keyboard;
use iced_core::image::Allocation;
use iced_winit::Proxy;
use iced_winit::futures::Runtime;
// use iced_winit::runtime::Action;
use iced_winit::winit;

// Servo
use dpi::PhysicalSize;
use servo::{JSValue, JavaScriptEvaluationError};
use servo::{WebView, WebViewId};

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::config::Config;
use crate::gui::program::Program;

use strum::EnumIter;

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

#[derive(Default, Clone, Debug)]
pub enum Mode {
    #[default]
    Normal,
    Visual,
    Follow,
    Command,
}
#[derive(Clone, Debug)]
pub enum Command {
    /// Search a pattern in the webpage
    /// When prefix is /
    PatternSearch(String),
    /// When prefix is :
    Action(String),
}
#[derive(Clone, Debug, EnumIter)]
pub enum Action {
    /// Search from keyword.
    Search(String),
    Url(String),
    VerticalSplit,
    HorizontalSplit,
}
impl ToString for Action {
    fn to_string(&self) -> String {
        match self {
            Action::Search(v) => "search".to_owned(),
            Action::Url(v) => "url".to_owned(),
            Action::VerticalSplit => "vsp".to_owned(),
            Action::HorizontalSplit => "hsp".to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Null,
    // Internal loop
    Tick,
    ServoNeedsResize(PhysicalSize<u32>),
    WindowCreated { window: Arc<winit::window::Window> },
    WebViewHasNewFrame(WebViewId),
    ImageAllocated(Allocation),
    Keyboard(keyboard::Event),

    // Actions
    FocusCommandLine(Mode),
    FocusView,
    TypingCommand(String),
    SubmitCommand,

    // Js
    DefaultJsEvaluate,
    TitleLoaded(Result<JSValue, JavaScriptEvaluationError>),
}

#[derive(Clone, Debug)]
pub enum UcerEvent {
    Wake,
    Message(Message),
}

#[derive(Default)]
pub enum StateType {
    #[default]
    Null,
    Init(Proxy<Message>),
    Ready(State),
}

/// The global state.
#[derive(Clone)]
pub struct State {
    pub config: Config,
    // Servo instance and state.
    pub servo: Rc<Servo>,
    // Proxy to send event to iced from servo runtime.
    pub proxy: Proxy<Message>,
}

/// Listen to events originating from the WebView.
/// Must be attached to the WebView uppon creation.
impl ::servo::WebViewDelegate for State {
    #[tracing::instrument(skip_all)]
    fn notify_new_frame_ready(&self, webview: WebView) {
        let message = Message::WebViewHasNewFrame(webview.id());
        self.proxy.send(message);
    }
}

pub fn runtime() -> anyhow::Result<
    Runtime<
        iced_futures::backend::native::tokio::Executor,
        futures::channel::mpsc::UnboundedSender<Message>,
        Message,
    >,
> {
    // Replace tokio support with std.
    // let executor = iced_futures::backend::native::thread_pool::Executor::new()?;
    let executor = iced_futures::backend::native::tokio::Executor::new()?;
    let (sender, _receiver) = futures::channel::mpsc::unbounded::<Message>();
    let runtime = iced_futures::Runtime::new(executor, sender);
    Ok(runtime)
}
