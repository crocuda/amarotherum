use super::{Message, Proxy};

// Servo
use dpi::PhysicalSize;
use euclid::{Box2D, Point2D, Scale, Size2D};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use servo::{DevicePixel, Servo, ServoBuilder};
use servo::{OffscreenRenderingContext, RenderingContext, WindowRenderingContext};

// Winit
use iced_winit::winit;
use winit::window::Window;

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

/// Return a state containing:
/// - servo,
/// - window/display handlers
/// - and a `Proxy` struct, channel communication for servo to send messages to iced,
///   through the user_events in the event_loop.
pub struct ServoState {
    pub window: Arc<Window>,
    pub servo: Servo,
    pub rendering_context: Rc<OffscreenRenderingContext>,
}
impl ServoState {
    /// Create the servo instance and pass it the required window/dispay handlers.
    /// Return a state containing:
    /// - servo,
    /// - window/display handlers
    /// - and a `Proxy` struct channel communication between iced and servo.
    pub fn new(window: &Arc<Window>, proxy: &mut Proxy) -> Rc<ServoState> {
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

        let servo = ServoBuilder::default()
            .event_loop_waker(Box::new(proxy.clone()))
            .build();
        servo.setup_logging();

        Rc::new(ServoState {
            window: window.clone(),
            servo,
            rendering_context,
        })
    }

    pub fn rect(&self) -> Box2D<i32, DevicePixel> {
        let size = self.rendering_context.size();
        // trace!("Size={:#?}", size);
        Box2D::new(
            Point2D::new(0, 0),
            Point2D::new(size.width as i32, size.height as i32),
        )
    }
}
