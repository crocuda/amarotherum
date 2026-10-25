use super::{Message, Proxy, RendererState, ServoState};
use crate::gui::components::App;

// Servo

// Wgpu
use iced_wgpu::graphics::{Shell, Viewport};
use iced_wgpu::{Engine, Renderer, wgpu};

// Winit
use iced_winit::core::time::Instant;
use iced_winit::core::{Event, Size, Theme};
use iced_winit::core::{mouse, renderer, shell, window};
use iced_winit::runtime::user_interface::{self, Cache, UserInterface};
use iced_winit::winit;
use winit::{
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::Window,
};

// Iced

// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use tracing::{debug, error, info, trace, warn};

pub struct IcedState {
    window: Arc<Window>,
    renderer: Renderer,

    app: App,

    events: Vec<Event>,
    messages: Vec<Message>,

    cursor: mouse::Cursor,
    cache: Cache,
    viewport: Viewport,
    modifiers: ModifiersState,
    resized: bool,
}

impl IcedState {
    pub fn new(render_state: &RendererState, servo_state: Rc<ServoState>, proxy: &Proxy) -> Self {
        let RendererState {
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
        let app = App::new(servo_state.clone(), proxy);

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
            cache: Cache::new(),
            viewport,
            resized: false,
        }
    }
    pub fn draw(&mut self, surface_texture: &wgpu::SurfaceTexture) {
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
