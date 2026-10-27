mod servo_state;
pub use servo_state::Servo;

// Winit
use iced::Event;
use iced_core::image::Allocation;
use iced_winit::futures::Runtime;
use iced_winit::winit;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

// Servo
use servo::{EventLoopWaker, WebView, WebViewId};

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    WindowCreated { window: Arc<winit::window::Window> },
    WebViewFrame(WebViewId),
    ImageAllocated(Allocation),
    Keyboard,
}

#[derive(Clone, Debug)]
pub enum UserEvent {
    Wake,
    Message(Message),
}

/// The global state.
#[derive(Clone)]
pub struct State {
    /// Iced and Servo need their own handler of the same window,
    /// so we put it in an Arc.
    /// The servo_state need to be accessed to draw frames.
    pub servo_state: Rc<Servo>,
}

/// Listen to events originating from the WebView.
/// Must be attached to the WebView uppon creation.
impl ::servo::WebViewDelegate for State {
    #[tracing::instrument(skip_all)]
    fn notify_new_frame_ready(&self, webview: WebView) {}
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
