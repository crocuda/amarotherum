use super::iced_state::IcedState;

// Wgpu
use iced_wgpu::graphics::{Shell, Viewport};
use iced_wgpu::{Engine, Renderer, wgpu};

// Winit
use iced_winit::winit;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

// Globals
use std::sync::{Arc, RwLock};

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

pub struct RendererState {
    pub window: Arc<Window>,

    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    config: wgpu::SurfaceConfiguration,

    is_surface_configured: bool,
}
impl RendererState {
    /// Redraw surfaces in offsceen buffer, and present to the screen.
    /// Redraws:
    /// - app
    /// - webviews
    #[tracing::instrument(skip_all)]
    async fn new(window: &Arc<Window>, event_loop: &ActiveEventLoop) -> Result<Self, AmaroError> {
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
        let RendererState {
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
