//! # MAIN LOOP
//!
//! Here we create the main event_oop responsible for window creation and event handling.
//! The complexity of running `iced` alongside `servo` lays and must remain in this file.
//!
//! Indeed, both libraries must share handlers from the same window, and that is
//! not possible if we create the window through `iced` helpers because `iced`
//! keeps the window struct private for the sake of simplicity.
//!
//! Thus we need to create our custom event_loop with `winit,
//! Which should be pretty close to the iced default, with the difference that we
//! pass down "externally" created window handlers to `iced` and `servo`.
//!
//! This way other files may only contain simple and composable iced or servo modules.
//!
//! ## Communication from iced to servo
//!
//! Iced widgets and elements methods directly interacts with WebViews.
//!
//! ## Communication from servo to iced (the other way around)
//!
//! Events emited by WebViews are caught by the `Proxy` which implements`WebViewDelegate`,
//! and passed down to the event_loop.

use crate::gui::state::{IcedState, Message, Proxy, RendererState, ServoState, UserEvent};

use iced_winit::conversion;
use iced_winit::core::mouse;
use iced_winit::core::shell;
use iced_winit::core::{Event, Size, Theme};
use iced_winit::futures::Runtime;
use iced_winit::winit;

// use iced_winit::futures::futures;
// same as,
use futures;
use std::thread;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::Window,
};

// Servo
use dpi::PhysicalSize;
use servo::RenderingContext;

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

/// Launch the event loop.
pub fn main() -> anyhow::Result<()> {
    // Initialize winit
    // let event_loop = EventLoop::with_user_event().build().unwrap();
    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut runner = Runner::new(&event_loop);
    // let mut runner_b = Runner::new(&event_loop);

    event_loop.run_app(&mut runner).unwrap();

    Ok(())
}
//

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
impl State {
    fn handle_key(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        code: KeyCode,
        is_pressed: bool,
    ) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }
}

enum Runner {
    Initial(Proxy),
    Ready { state: State },
}

impl Runner {
    fn new(event_loop: &EventLoop<UserEvent>) -> Self {
        Self::Initial(Proxy::new(event_loop))
    }
    // fn runtime() -> Runtime<Executor, Message, Action> {
    //     let executor = tokio::runtime::Runtime::new()?;
    // }
    fn executor() -> anyhow::Result<()> {
        // Replace tokio support with std.
        // let executor = iced_futures::backend::native::thread_pool::Executor::new()?;
        let executor = iced_futures::backend::native::tokio::Executor::new()?;

        let (sender, _receiver) = futures::channel::mpsc::unbounded::<Message>();
        let runtime = iced_futures::Runtime::new(executor, sender);
        Ok(())
    }
}
impl ApplicationHandler<UserEvent> for Runner {
    #[tracing::instrument(skip_all)]
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // You should change this if you want to render continuously
        event_loop.set_control_flow(ControlFlow::Wait);

        if let Self::Initial(proxy) = self {
            // Create headless window for servo.
            let window = Arc::new(
                event_loop
                    .create_window(winit::window::WindowAttributes::default())
                    .expect("Create invisible servo window"),
            );
            // Pass required window/display handlers to servo.
            let servo_state = ServoState::new(&window, proxy);
            let render_state =
                futures::executor::block_on(RendererState::new(&window, event_loop)).unwrap();

            // Initialize iced
            let iced_state = IcedState::new(&render_state, servo_state.clone(), proxy);

            let state = State {
                window: window.clone(),
                servo_state: servo_state.clone(),
                render_state: render_state,
                iced_state,

                proxy: proxy.clone(),
            };

            *self = Self::Ready { state };
        }
    }

    #[tracing::instrument(skip_all)]
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        let Self::Ready {
            state,
            // State {
            //     iced_state,
            //     servo_state,
            //     ..
            // },
        } = self
        else {
            return;
        };
        debug!("UserEvent={:#?}", event);
        // state.servo_state.servo.spin_event_loop();
        match event {
            UserEvent::Message(message) => {
                state.iced_state.app.update(message);
                // match message {
                //     Message::WebViewFrame(id) => {
                //
                //         state.iced_state.messages.push(Message::WebViewFrame(id));
                //     }
                //     _ => {}
                // };
            }
            _ => {}
        };
        state.render_state.render(&mut state.iced_state);
        // Push messages originating from user_event to channel
        // messages.iter().for_each(|e| {
        //     message_channel.push(e.clone());
        // });
        // messages.clear();
        // state.servo_state.servo.spin_event_loop();
    }

    /// Converts a winit window event into an iced event.
    #[tracing::instrument(skip_all)]
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Self::Ready { state } = self else {
            return;
        };
        debug!("WindowEvent={:#?}", event);
        state.servo_state.servo.spin_event_loop();

        let State {
            window,
            servo_state,
            render_state,
            iced_state,
            ..
        } = state;

        // You should wire your own ticker logic if you will use widgets that must
        // notify the runtime concurrently.
        let waker = shell::Waker::noop();

        match event {
            WindowEvent::RedrawRequested => {
                // if *resized {
                //     let size = window.inner_size();
                //     *viewport = Viewport::with_physical_size(
                //         Size::new(size.width, size.height),
                //         renderer::Scale {
                //             window: window.scale_factor() as f32,
                //             application: 1.0,
                //         },
                //     );
                //     render_state.resize(size.width, size.height);
                //
                //     *resized = false;
                // }

                render_state.render(iced_state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let IcedState {
                    cursor, viewport, ..
                } = iced_state;
                *cursor = mouse::Cursor::Available(conversion::cursor_position(
                    position,
                    viewport.scale_factor(),
                ));
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                let IcedState { modifiers, .. } = iced_state;
                *modifiers = new_modifiers.state();
            }
            WindowEvent::Resized(_) => {
                // R
                let IcedState { resized, .. } = iced_state;
                *resized = true;
                // Resize servo window
                let size = servo_state.window.inner_size();
                servo_state
                    .rendering_context
                    .resize(PhysicalSize::new(size.width, size.height));

                // window.request_redraw();
                render_state.render(iced_state);
            }
            WindowEvent::KeyboardInput {
                device_id,
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                // state.handle_key(event_loop, code, key_state.is_pressed());
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }

        let IcedState {
            window,
            renderer,
            app,
            events,
            messages,
            cursor,
            cache,
            viewport,
            modifiers,
            resized,
        } = iced_state;

        // Map window event to iced event
        if let Some(event) =
            conversion::window_event(event, window.scale_factor() as f32, *modifiers)
        {
            events.push(event);
        }

        // If there are events pending
        //     if !events.is_empty() {
        //         // User Interface
        //         // We process them
        //         let mut interface = UserInterface::build(
        //             app.view(),
        //             viewport.logical_size(),
        //             std::mem::take(cache),
        //             renderer,
        //         );
        //
        //         let mut message_channel: shell::Bus<Message> = shell::Bus::new();
        //
        //         let _ = interface.update(
        //             &window,
        //             &waker,
        //             &events,
        //             *cursor,
        //             renderer,
        //             &mut message_channel,
        //         );
        //
        //         events.clear();
        //         *cache = interface.into_cache();
        //
        //         // update our UI with any messages
        //         for message in message_channel {
        //             app.update(message);
        //             // and request a redraw
        //         }
        //         // window.request_redraw();
        //     }
        render_state.render(iced_state);
    }
}
