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

use crate::gui::components::Message;

use iced_wgpu::graphics::{Shell, Viewport};
use iced_wgpu::{Engine, Renderer, wgpu};
use iced_winit::conversion;
use iced_winit::core::mouse;
use iced_winit::core::renderer;
use iced_winit::core::shell;
use iced_winit::core::time::Instant;
use iced_winit::core::window;
use iced_winit::core::{Event, Size, Theme};
use iced_winit::runtime::user_interface::{self, UserInterface};
use iced_winit::winit;

use futures;
// use iced_winit::futures::futures;

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
use euclid::{Box2D, Point2D, Scale, Size2D};
use indexmap::IndexMap;
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use servo::{
    DevicePixel, Servo, ServoBuilder, WebView, WebViewBuilder, WebViewDelegate, WebViewId,
};
use servo::{
    EventLoopWaker, OffscreenRenderingContext, RenderingContext, SoftwareRenderingContext,
    WindowRenderingContext,
};

// Iced macros
use iced::widget::{column, container, row, text};
use iced::{
    Element, Fill, Subscription, Task, Widget,
    widget::{Column, Container, Image, Row, Text, text_input},
};

// Globals
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use tracing::{debug, error, info, trace, warn};

// Error
use amarotherum_error::AmaroError;

use crate::gui::components;

/// Launch the event loop.
pub fn main() -> anyhow::Result<()> {
    // Initialize winit
    // let event_loop = EventLoop::with_user_event().build().unwrap();
    let event_loop = EventLoop::with_user_event().build().unwrap();
    let mut runner = Runner::new(&event_loop);

    event_loop.run_app(&mut runner).unwrap();

    Ok(())
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
    render_state: RenderState,
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
    fn new(window: &Arc<Window>, proxy: &mut Proxy) -> Rc<ServoState> {
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

struct RenderState {
    pub window: Arc<Window>,

    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    config: wgpu::SurfaceConfiguration,

    is_surface_configured: bool,
}
impl RenderState {
    /// Redraw surfaces in offsceen buffer, and present to the screen.
    /// Redraws:
    /// - app
    /// - webviews
    #[tracing::instrument(skip_all)]
    async fn new(
        window: &Arc<Window>,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Self, AmaroError> {
        // The instance is a handle(builder) to our GPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or_default(),
            // ..wgpu::InstanceDescriptor::new_without_display_handle()
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
                event_loop.owned_display_handle(),
            ))
        });

        // The part of the window we draw to.
        let surface = instance
            .create_surface(window.clone())
            .expect("create window surface");
        // A convenient interface/handler to our GPU.
        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface))
            .await
            .expect("create adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: adapter.features() & wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .expect("request device");

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .expect("Get preferred format");

        // Create surface configuration.
        let physical_size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width,
            height: physical_size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        Ok(Self {
            window: window.clone(),
            adapter,
            surface,
            device,
            queue,
            config,
            format,
            is_surface_configured: true,
        })
    }

    // Resize the drawing surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }
    pub fn render(&mut self, iced_state: &mut IcedState) {
        let RenderState {
            surface,
            device,
            config,
            ..
        } = self;
        let surface_texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(v)
            | wgpu::CurrentSurfaceTexture::Suboptimal(v) => v,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&device, &config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                return;
            }
        };
        // Draw iced app
        iced_state.draw(&surface_texture);
        // Present the frame
        self.queue.present(surface_texture);
    }
}

struct IcedState {
    window: Arc<Window>,
    renderer: Renderer,

    app: components::App,

    events: Vec<Event>,
    messages: Vec<Message>,

    cursor: mouse::Cursor,
    cache: user_interface::Cache,
    viewport: Viewport,
    modifiers: ModifiersState,
    resized: bool,
}
impl IcedState {
    fn new(render_state: &RenderState, servo_state: Rc<ServoState>, proxy: &Proxy) -> Self {
        let RenderState {
            window,
            adapter,
            device,
            queue,
            format,
            ..
        } = render_state;
        let renderer = Renderer::new(
            Engine::new(
                adapter,
                device.clone(),
                queue.clone(),
                format.clone(),
                None,
                Shell::headless(),
            ),
            renderer::Settings::default(),
        );

        // Initialize scene and GUI controls
        let app = components::App::new(servo_state.clone(), proxy);

        // Set viewport size
        let physical_size = window.inner_size();
        let viewport = Viewport::with_physical_size(
            Size::new(physical_size.width, physical_size.height),
            renderer::Scale {
                window: window.scale_factor() as f32,
                application: 1.0,
            },
        );

        Self {
            window: window.clone(),
            renderer,

            app,
            events: Vec::new(),
            messages: Vec::new(),

            cursor: mouse::Cursor::Unavailable,
            modifiers: ModifiersState::default(),
            cache: user_interface::Cache::new(),
            viewport,
            resized: false,
        }
    }
    fn draw(&mut self, surface_texture: &wgpu::SurfaceTexture) {
        let IcedState {
            window,
            renderer,

            app,

            viewport,
            cursor,
            cache,
            ..
        } = self;

        // You should wire your own ticker logic if you will use widgets that must
        // notify the runtime concurrently.
        let waker = shell::Waker::noop();

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Draw iced on top
        let mut interface = UserInterface::build(
            app.view(),
            viewport.logical_size(),
            std::mem::take(cache),
            renderer,
        );
        let (state, _) = interface.update(
            &window,
            &waker,
            &[Event::Window(
                window::Event::RedrawRequested(Instant::now()),
            )],
            *cursor,
            renderer,
            &mut shell::Bus::<Message>::new(),
        );
        if let user_interface::State::Updated {
            mouse_interaction, ..
        } = state
        {
            // Update the mouse cursor
            if let Some(icon) = iced_winit::conversion::mouse_interaction(mouse_interaction) {
                window.set_cursor(icon);
                window.set_cursor_visible(true);
            } else {
                window.set_cursor_visible(false);
            }
        }
        interface.draw(renderer, &Theme::Dark, &renderer::Style::default(), *cursor);
        *cache = interface.into_cache();
        renderer.present(
            None,
            surface_texture.texture.format().clone(),
            &view,
            &viewport,
        );
    }
}

#[derive(Clone, Debug)]
enum UserEvent {
    Wake,
    Message(Message),
}

/// The proxy is for sending events from servo to iced, to request redraws
/// when new frames can be displayed.
#[derive(Clone)]
pub struct Proxy(winit::event_loop::EventLoopProxy<UserEvent>);
impl Proxy {
    fn new(event_loop: &EventLoop<UserEvent>) -> Self {
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

pub fn winit_size_to_euclid_size<T>(size: PhysicalSize<T>) -> Size2D<T, DevicePixel> {
    Size2D::new(size.width, size.height)
}

enum Runner {
    Initial(Proxy),
    Ready { state: State },
}

impl Runner {
    fn new(event_loop: &EventLoop<UserEvent>) -> Self {
        Self::Initial(Proxy::new(event_loop))
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
                futures::executor::block_on(RenderState::new(&window, event_loop)).unwrap();

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
        let Self::Ready { state } = self else {
            return;
        };
        // state.servo_state.servo.spin_event_loop();
        debug!("UserEvent={:#?}", event);
        match event {
            UserEvent::Message(message) => {
                match message {
                    Message::WebViewFrame(id) => {
                        state.iced_state.messages.push(Message::WebViewFrame(id));
                    }
                    _ => {}
                };
            }
            _ => {}
        };
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

                window.request_redraw();
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
        // if !events.is_empty() {
        // User Interface
        // We process them
        let mut interface = UserInterface::build(
            app.view(),
            viewport.logical_size(),
            std::mem::take(cache),
            renderer,
        );

        let mut message_channel: shell::Bus<Message> = shell::Bus::new();

        let _ = interface.update(
            &window,
            &waker,
            &events,
            *cursor,
            renderer,
            &mut message_channel,
        );

        events.clear();
        *cache = interface.into_cache();

        // Push messages originating from user_event to channel
        messages.iter().for_each(|e| {
            message_channel.push(e.clone());
        });
        messages.clear();

        // update our UI with any messages
        for message in message_channel {
            app.update(message);
            // and request a redraw
        }
        window.request_redraw();
        // }
    }
}
