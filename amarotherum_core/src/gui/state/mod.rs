mod servo_state;
pub use servo_state::Servo;

// Winit
use iced::Event;
use iced::keyboard;
use iced_core::image::Allocation;
use iced_winit::Proxy;
use iced_winit::futures::Runtime;
use iced_winit::runtime::Action;
use iced_winit::winit;

use crate::gui::program::Program;

// Servo
use dpi::PhysicalSize;
use servo::{JSValue, JavaScriptEvaluationError};
use servo::{WebView, WebViewId};

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    ServoNeedsResize(PhysicalSize<u32>),
    WindowCreated { window: Arc<winit::window::Window> },
    WebViewHasNewFrame(WebViewId),
    ImageAllocated(Allocation),
    Keyboard(keyboard::Event),

    // Js
    DefaultJsEvaluate,
    TitleLoaded(Result<JSValue, JavaScriptEvaluationError>),
}

#[derive(Clone, Debug)]
pub enum UserEvent {
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
