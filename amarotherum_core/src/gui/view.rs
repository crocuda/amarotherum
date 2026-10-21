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
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use servo::{DevicePixel, Servo, ServoBuilder, WebView, WebViewBuilder, WebViewDelegate};
use servo::{EventLoopWaker, RenderingContext, SoftwareRenderingContext, WindowRenderingContext};
use url::Url;

// Iced macros
use iced::widget::{column, container, row, text};
use iced::{
    Element, Fill, Subscription, Task,
    widget::{Column, Container, Image, Row, Text, text_input},
};

// Globals
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use tracing::{debug, error, info, trace, warn};

// Error
use amarotherum_error::AmaroError;
use miette::Result;

pub fn main() -> Result<(), AmaroError> {
    // tracing_subscriber::fmt::init();

    // Initialize winit
    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).unwrap();

    Ok(())
}

pub enum Message {
    Keyboard,
}

struct ServoState {
    window: Arc<Window>,
    servo: Servo,
    rendering_context: Rc<WindowRenderingContext>,
    webviews: RefCell<Vec<WebView>>,
}
impl ServoState {
    fn rect(&self) -> Box2D<i32, DevicePixel> {
        let size = self.rendering_context.size();
        trace!("Size={:#?}", size);
        Box2D::new(
            Point2D::new(0, 0),
            Point2D::new(size.width as i32, size.height as i32),
        )
    }
}
impl ::servo::WebViewDelegate for ServoState {
    fn notify_new_frame_ready(&self, _: WebView) {
        self.window.request_redraw();
    }
}
struct State {
    window: Arc<Window>,
    queue: wgpu::Queue,
    device: wgpu::Device,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    renderer: Renderer,
    events: Vec<Event>,
    cursor: mouse::Cursor,
    cache: user_interface::Cache,
    viewport: Viewport,
    modifiers: ModifiersState,
    resized: bool,
}
enum App {
    Initial(Waker),
    Ready {
        state: State,
        servo_state: Rc<ServoState>,
    },
}

impl App {
    fn new(event_loop: &EventLoop<WakerEvent>) -> Self {
        Self::Initial(Waker::new(event_loop))
    }
}
impl ApplicationHandler<WakerEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Self::Initial(waker) = self {
            // Create headless window for servo.
            let window = Arc::new(
                event_loop
                    .create_window(winit::window::WindowAttributes::default().with_visible(true))
                    .expect("Create invisible servo window"),
            );
            // Initial window size
            let width = 1500;
            let height = 1500;
            let initial_size = PhysicalSize::new(width, height);
            let size = PhysicalSize::new(window.inner_size().width, window.inner_size().height);

            let display_handle = window.display_handle().unwrap();
            let window_handle = window.window_handle().unwrap();
            let rendering_context =
                Rc::new(WindowRenderingContext::new(display_handle, window_handle, size).unwrap());
            let _ = rendering_context.make_current();

            let servo = ServoBuilder::default()
                .event_loop_waker(Box::new(waker.clone()))
                .build();
            servo.setup_logging();
            let servo_state = Rc::new(ServoState {
                window: window,
                servo,
                rendering_context,
                webviews: Default::default(),
            });
            // Make a new WebView and assign the `AppState` as the delegate.
            let url = Url::parse("https://demo.servo.org/").expect("Guaranteed by argument");

            let webview =
                WebViewBuilder::new(&servo_state.servo, servo_state.rendering_context.clone())
                    .url(url)
                    .hidpi_scale_factor(Scale::new(servo_state.window.scale_factor() as f32))
                    .delegate(servo_state.clone())
                    .build();

            servo_state.webviews.borrow_mut().push(webview);

            // Create main window
            let window = Arc::new(
                event_loop
                    .create_window(winit::window::WindowAttributes::default())
                    .expect("Create window"),
            );
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

            // You should change this if you want to render continuously
            event_loop.set_control_flow(ControlFlow::Wait);

            let state = State {
                window,
                device,
                queue,
                renderer,
                surface,
                format,
                events: Vec::new(),
                cursor: mouse::Cursor::Unavailable,
                modifiers: ModifiersState::default(),
                cache: user_interface::Cache::new(),
                viewport,
                resized: false,
            };

            *self = Self::Ready { state, servo_state };
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, _event: WakerEvent) {
        if let Self::Ready { state, servo_state } = self {
            servo_state.servo.spin_event_loop();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.servo_window_event(event_loop, _window_id, event.clone());

        let Self::Ready {
            state,
            servo_state: _,
        } = self
        else {
            return;
        };
        let State {
            window,
            device,
            queue,
            surface,
            format,
            renderer,
            events,
            viewport,
            cursor,
            modifiers,
            cache,
            resized,
            ..
        } = state;
        //Exit if not event do not originate from the main window.
        if _window_id != window.id() {
            return;
        }

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

                        let windows = container("windows")
                            .padding(5)
                            .align_top(Fill)
                            .align_left(Fill)
                            .style(container::bordered_box);
                        let statusline = container("statusline");
                        let commandline = container("commandline");

                        // Draw iced on top
                        let mut interface = UserInterface::build(
                            column![windows, statusline, commandline],
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
                        window.request_redraw();
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
                *resized = true;
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
        if !events.is_empty() {
            let windows = container("windows")
                .padding(5)
                .align_top(Fill)
                .align_left(Fill)
                .style(container::bordered_box);
            let statusline = container("statusline");
            let commandline = container("commandline");
            // We process them
            let mut interface = UserInterface::build(
                column![windows, statusline, commandline],
                viewport.logical_size(),
                std::mem::take(cache),
                renderer,
            );

            let mut messages: shell::Bus<Message> = shell::Bus::new();

            let _ = interface.update(&window, &waker, &events, *cursor, renderer, &mut messages);

            events.clear();
            *cache = interface.into_cache();

            // update our UI with any messages
            for message in messages {}

            // and request a redraw
            window.request_redraw();
        }
    }
}

impl App {
    fn servo_window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Self::Ready {
            state: _,
            servo_state: state,
        } = self
        else {
            return;
        };

        //Exit if not event do not originate from the servo window.
        if _window_id != state.window.id() {
            return;
        }

        state.servo.spin_event_loop();
        // debug!("window_id={:#?}", _window_id);
        // debug!("event={:#?}", event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let webview = state.webviews.borrow().last().unwrap().paint();
                // webview.paint();
                debug!(
                    "LoadStatus={:#?}",
                    state.webviews.borrow().last().unwrap().load_status()
                );
                // println!("LoadStatus={:#?}", webview.load_status());
                // state.rendering_context.present();

                let rect = state.rect();
                let image_buffer = state.rendering_context.read_to_image(rect.clone()).unwrap();
                println!("{:?}", image_buffer);
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
struct Waker(winit::event_loop::EventLoopProxy<WakerEvent>);
#[derive(Debug)]
struct WakerEvent;

impl Waker {
    fn new(event_loop: &EventLoop<WakerEvent>) -> Self {
        Self(event_loop.create_proxy())
    }
}

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }

    fn wake(&self) {
        if let Err(error) = self.0.send_event(WakerEvent) {
            warn!(?error, "Failed to wake event loop");
        }
    }
}

pub fn winit_size_to_euclid_size<T>(size: PhysicalSize<T>) -> Size2D<T, DevicePixel> {
    Size2D::new(size.width, size.height)
}
