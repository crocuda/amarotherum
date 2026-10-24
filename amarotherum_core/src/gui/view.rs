//! MAIN LOOP
//!
//! In this file we create the main Loop,responsible for window creation and event handling.
//!
//! The complexity of running `iced` alongside `servo` lays and must remain in this file.
//!
//! Indeed, both libraries must share the same window handler, and that is only possible
//! if we create a custom loop (not the iced default) through winit
//! and then pass down a shared window handler to iced and servo.
//!
//! This way other files may only contain simple and composable iced or servo modules.

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
use iced_winit::futures;
use iced_winit::runtime::user_interface::{self, UserInterface};
use iced_winit::winit;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    keyboard::ModifiersState,
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
use miette::Result;

use crate::gui::components;

/// Launch the event loop.
pub fn main() -> Result<(), AmaroError> {
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
    /// The servo_state need to be accessed to draw frames.
    pub servo_state: Rc<ServoState>,
}
pub struct ServoState {
    pub window: Arc<Window>,
    pub servo: Servo,
    pub rendering_context: Rc<OffscreenRenderingContext>,
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

struct IcedState {
    app: components::App,
    queue: wgpu::Queue,
    device: wgpu::Device,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    renderer: Renderer,

    events: Vec<Event>,
    messages: Vec<Message>,

    cursor: mouse::Cursor,
    cache: user_interface::Cache,
    viewport: Viewport,
    modifiers: ModifiersState,
    resized: bool,
}

#[derive(Clone, Debug)]
enum UserEvent {
    Wake,
    NewFrameReady,
    Message(Message),
}

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
    fn servo_init(window: &Arc<Window>, proxy: &mut Proxy) -> Rc<ServoState> {
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
}
impl ApplicationHandler<UserEvent> for Runner {
    #[tracing::instrument(skip_all)]
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Self::Initial(proxy) = self {
            // Create headless window for servo.
            let window = Arc::new(
                event_loop
                    .create_window(winit::window::WindowAttributes::default())
                    .expect("Create invisible servo window"),
            );
            let servo_state = Runner::servo_init(&window, proxy);

            let physical_size = window.inner_size();
            let viewport = Viewport::with_physical_size(
                Size::new(physical_size.width, physical_size.height),
                renderer::Scale {
                    window: window.scale_factor() as f32,
                    application: 1.0,
                },
            );

            let backend = wgpu::Backends::from_env().unwrap_or_default();
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: backend,
                ..wgpu::InstanceDescriptor::new_without_display_handle()
            });
            let surface = instance
                .create_surface(window.clone())
                .expect("Create window surface");

            let (format, adapter, device, queue) = futures::futures::executor::block_on(async {
                let adapter =
                    wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface))
                        .await
                        .expect("Create adapter");

                let adapter_features = adapter.features();

                let capabilities = surface.get_capabilities(&adapter);

                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        label: None,
                        required_features: adapter_features & wgpu::Features::default(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                        trace: wgpu::Trace::Off,
                        experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    })
                    .await
                    .expect("Request device");

                (
                    capabilities
                        .formats
                        .iter()
                        .copied()
                        .find(wgpu::TextureFormat::is_srgb)
                        .or_else(|| capabilities.formats.first().copied())
                        .expect("Get preferred format"),
                    adapter,
                    device,
                    queue,
                )
            });

            surface.configure(
                &device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format,
                    width: physical_size.width,
                    height: physical_size.height,
                    present_mode: wgpu::PresentMode::AutoVsync,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                    color_space: wgpu::SurfaceColorSpace::Auto,
                },
            );

            // Initialize iced
            let renderer = {
                let engine = Engine::new(
                    &adapter,
                    device.clone(),
                    queue.clone(),
                    format,
                    None,
                    Shell::headless(),
                );
                Renderer::new(engine, renderer::Settings::default())
            };

            // Initialize scene and GUI controls
            let app = components::App::new(servo_state.clone(), proxy);

            // You should change this if you want to render continuously
            event_loop.set_control_flow(ControlFlow::Wait);
            let iced_state = IcedState {
                app,
                device,
                queue,
                renderer,
                surface,
                format,

                events: Vec::new(),
                messages: Vec::new(),

                cursor: mouse::Cursor::Unavailable,
                modifiers: ModifiersState::default(),
                cache: user_interface::Cache::new(),
                viewport,
                resized: false,
            };

            let state = State {
                window,
                iced_state,
                servo_state: servo_state.clone(),
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
            iced_state:
                IcedState {
                    app,
                    device,
                    queue,
                    surface,
                    format,
                    renderer,

                    events,
                    messages,

                    viewport,
                    cursor,
                    modifiers,
                    cache,
                    resized,
                },
            ..
        } = state;

        // You should wire your own ticker logic if you will use widgets that must
        // notify the runtime concurrently.
        let waker = shell::Waker::noop();

        match event {
            WindowEvent::RedrawRequested => {
                if *resized {
                    let size = window.inner_size();

                    *viewport = Viewport::with_physical_size(
                        Size::new(size.width, size.height),
                        renderer::Scale {
                            window: window.scale_factor() as f32,
                            application: 1.0,
                        },
                    );

                    surface.configure(
                        &device,
                        &wgpu::SurfaceConfiguration {
                            format: *format,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::AutoVsync,
                            alpha_mode: wgpu::CompositeAlphaMode::Auto,
                            view_formats: vec![],
                            desired_maximum_frame_latency: 2,
                            color_space: wgpu::SurfaceColorSpace::Auto,
                        },
                    );

                    *resized = false;
                }

                match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) => {
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: None,
                            });

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

                        // Update the mouse cursor
                        if let user_interface::State::Updated {
                            mouse_interaction, ..
                        } = state
                        {
                            // Update the mouse cursor
                            if let Some(icon) =
                                iced_winit::conversion::mouse_interaction(mouse_interaction)
                            {
                                window.set_cursor(icon);
                                window.set_cursor_visible(true);
                            } else {
                                window.set_cursor_visible(false);
                            }
                        }

                        // Draw the interface
                        interface.draw(
                            renderer,
                            &Theme::Dark,
                            &renderer::Style::default(),
                            *cursor,
                        );
                        *cache = interface.into_cache();

                        renderer.present(None, frame.texture.format(), &view, &viewport);

                        // Present the frame
                        queue.present(frame);
                    }
                    _ => {
                        // Try rendering again next frame.
                        // BUG: Causes an update loop and freeze app.
                        // window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                *cursor = mouse::Cursor::Available(conversion::cursor_position(
                    position,
                    viewport.scale_factor(),
                ));
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                *modifiers = new_modifiers.state();
            }
            WindowEvent::Resized(_) => {
                let size = servo_state.window.inner_size();
                servo_state
                    .rendering_context
                    .resize(PhysicalSize::new(size.width, size.height));

                *resized = true;
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }

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
