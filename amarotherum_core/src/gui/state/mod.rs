mod iced_state;
mod renderer_state;
mod servo_state;

pub use iced_state::IcedState;
pub use renderer_state::RendererState;
pub use servo_state::ServoState;

// Winit
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
    WebViewFrame(WebViewId),
    Keyboard,
}

#[derive(Clone, Debug)]
pub enum UserEvent {
    Wake,
    Message(Message),
}

/// The global state.
pub struct State {
    /// Iced and Servo need their own handler of the same window,
    /// so we put it in an Arc.
    window: Arc<Window>,

    /// The proxy is for passing messages from iced to servo
    /// through the event loop.
    pub proxy: Proxy,

    iced_state: IcedState,
    render_state: RendererState,
    /// The servo_state need to be accessed to draw frames.
    pub servo_state: Rc<ServoState>,
}

/// The proxy is for sending events from servo to iced, to request redraws
/// when new frames can be displayed.
#[derive(Clone)]
pub struct Proxy(winit::event_loop::EventLoopProxy<UserEvent>);
impl Proxy {
    pub fn new(event_loop: &EventLoop<UserEvent>) -> Self {
        Self(event_loop.create_proxy())
    }
}
impl EventLoopWaker for Proxy {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }
    fn wake(&self) {
        if let Err(error) = self.0.send_event(UserEvent::Wake) {
            warn!(?error, "Failed to wake event loop");
        }
    }
}
/// Listen to events originating from the WebView.
/// Must be attached to the WebView uppon creation.
impl ::servo::WebViewDelegate for Proxy {
    #[tracing::instrument(skip_all)]
    fn notify_new_frame_ready(&self, webview: WebView) {
        let event = UserEvent::Message(Message::WebViewFrame(webview.id()));
        if let Err(error) = self.0.send_event(event) {
            warn!(?error, "Failed to notify new frame ready");
        } else {
            info!("notify new frame.")
        }
    }
}
