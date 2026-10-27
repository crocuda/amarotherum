//! Clone/Rework of [iced_webview_v2](https://github.com/franzos/iced_webview_v2.git)
//!
//! Orignal devs:
//! - LegitCamper
//! - franzos
//!
//! LICENCE: Apache Licence 2.0
//!
//! Clone/Rework of [iced_blocks](https://github.com/n1ght-hunter/iced_blocks.git)
//!
//! Orignal devs:
//! - n1ght-hunter
//!
//! LICENCE: MIT Licence

use super::Message;
use tokio::sync::Notify;

// Servo
use dpi::PhysicalSize;
use euclid::{Box2D, Point2D, Scale, Size2D};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use servo::{DevicePixel, Servo as ServoInstance, ServoBuilder, WebView, WebViewDelegate};
use servo::{OffscreenRenderingContext, RenderingContext, WindowRenderingContext};

// Winit
use iced::Subscription;
use iced_winit::winit;
use winit::window::Window;

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

#[derive(Clone)]
pub struct ServoState {
    pub window: Arc<Window>,
    pub rendering_context: Rc<OffscreenRenderingContext>,
}
impl ::servo::WebViewDelegate for ServoState {
    fn notify_new_frame_ready(&self, _: WebView) {
        // self.window.request_redraw();
    }
}
impl ServoState {
    pub fn rect(&self) -> Box2D<i32, DevicePixel> {
        let size = self.rendering_context.size();
        // trace!("Size={:#?}", size);
        Box2D::new(
            Point2D::new(0, 0),
            Point2D::new(size.width as i32, size.height as i32),
        )
    }
}
/// Return a state containing:
/// - servo,
/// - window/display handlers
/// - and a `Proxy` struct, channel communication for servo to send messages to iced,
///   through the user_events in the event_loop.
#[derive(Clone)]
pub struct Servo {
    pub state: ServoState,
    pub instance: ServoInstance,
}
impl Servo {
    /// Create the servo instance and pass it the required window/dispay handlers.
    /// Return a state containing:
    /// - servo,
    /// - window/display handlers
    /// - and a `Proxy` struct channel communication between iced and servo.
    pub fn try_new(window: &Arc<Window>) -> anyhow::Result<Self> {
        // Initial window size
        let inner_size = window.inner_size();
        let size = PhysicalSize::new(inner_size.width, inner_size.height);

        let display_handle = window.display_handle().unwrap();
        let window_handle = window.window_handle().unwrap();

        let rendering_context =
            Rc::new(WindowRenderingContext::new(display_handle, window_handle, size).unwrap());
        let rendering_context = Rc::new(WindowRenderingContext::offscreen_context(
            &rendering_context,
            size,
        ));
        let _ = rendering_context.make_current();

        let notify = Arc::new(Notify::new());
        let waker = ServoWaker {
            notify: notify.clone(),
        };
        let instance = ServoBuilder::default()
            .event_loop_waker(Box::new(waker))
            .build();

        let state = ServoState {
            window: window.clone(),
            rendering_context,
        };

        Ok(Servo { state, instance })
    }
}

/// Event-driven waker that Servo calls (from any thread) whenever it wants the
/// embedder to spin its event loop. The `Notify` coalesces multiple wake signals
/// into a single pending notification, so a burst of calls produces at most one
/// wake-up on the consumer side.
#[derive(Clone)]
struct ServoWaker {
    notify: Arc<Notify>,
}
impl servo::EventLoopWaker for ServoWaker {
    fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
        Box::new(self.clone())
    }
    fn wake(&self) {
        self.notify.notify_one();
    }
}
