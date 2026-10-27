use crate::gui::state::{Message, Servo};

use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{ContentFit, Length, Task};
use iced_wgpu::Renderer;
use iced_winit::core::{Color, Theme};

// Images
// use iced::advanced::image::{Image, allocate};
use iced::widget::image::{Allocation, FilterMethod, Handle};
use iced::widget::image::{Image, allocate};
use iced_core::Bytes;
use iced_core::image::Handle::Rgba;
use image::DynamicImage;

// Servo
use euclid::Scale;
use servo::{OffscreenRenderingContext, RenderingContext};
use servo::{WebView, WebViewBuilder, WebViewId};
use url::Url;

use std::fs;
// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use tracing::{debug, error, info, trace, warn};
// Error
use amarotherum_error::AmaroError;
use miette::Result;

#[derive(Clone)]
pub struct Tab {
    servo: Rc<Servo>,
    webview: WebView,
    // last_frame: Arc<RwLock<Handle>>,
    last_frame: Option<Allocation>,
}

impl Tab {
    pub fn new(servo: Rc<Servo>) -> Self {
        let url = Url::parse("https://elrumordelaluz.github.io/reshake/").unwrap();
        // let url = Url::parse("https://demo.servo.org").expect("Guaranteed by argument");
        let webview = WebViewBuilder::new(&servo.instance, servo.state.rendering_context.clone())
            .url(url)
            .hidpi_scale_factor(Scale::new(servo.state.window.clone().scale_factor() as f32))
            .delegate(Rc::new(servo.state.clone()))
            .build();

        // let bytes = fs::read("/home/anon/Pictures/megatron_pp.png").unwrap();

        Self {
            servo,
            webview,
            // last_frame: Arc::new(RwLock::new(Handle::from_bytes(""))),
            // last_frame: Handle::from_bytes(bytes),
            last_frame: None,
        }
    }
}

impl Tab {
    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, message: Message) -> iced_runtime::Task<Message> {
        match message {
            Message::Tick => {
                if let Ok(image) = webview_to_image(self, self.servo.clone()) {
                    image
                    // *self.last_frame.write().unwrap() = image;
                } else {
                    Task::none()
                }
            }
            Message::WebViewFrame(_) => Task::none(),
            Message::ImageAllocated(allocation) => {
                self.last_frame = Some(allocation);
                Task::none()
            }
            _ => Task::none(),
        }
    }
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&self) -> impl Widget<Message, Theme, Renderer> + 'a {
        debug!("view tab");

        if let Some(allocation) = self.last_frame.clone() {
            let image: Image<Handle> = Image::new(allocation.handle())
                .filter_method(FilterMethod::Nearest)
                .content_fit(ContentFit::Fill)
                .expand(true);
            column![
                container(image)
                    .align_top(Length::Fill)
                    .align_left(Length::Fill)
                    .style(container::bordered_box),
            ]
        } else {
            column![
                container("empty")
                    .align_top(Length::Fill)
                    .align_left(Length::Fill)
                    .style(container::bordered_box),
            ]
        }
        // let image: Image<Handle> = Image::new("/home/anon/Pictures/megatron_pp.png")
    }
}

/// Turn the last frame of a servo webview into an image that can be printed by iced.
#[tracing::instrument(skip_all)]
// pub fn webview_to_image(tab: &Tab, servo: Rc<Servo>) -> Result<Handle, AmaroError> {
// pub fn webview_to_image(tab: &Tab, servo: Rc<Servo>) -> Result<Bytes, AmaroError> {
pub fn webview_to_image(tab: &Tab, servo: Rc<Servo>) -> Result<Task<Message>> {
    debug!("LoadStatus={:#?}", tab.webview.load_status());
    let _ = servo.state.rendering_context.make_current();
    let _ = servo.state.rendering_context.prepare_for_rendering();
    tab.webview.paint();

    // Servo image
    let rect = servo.state.rect();
    // let image =
    //     DynamicImage::ImageRgba8(servo.state.rendering_context.read_to_image(rect).unwrap());
    //
    let image = servo.state.rendering_context.read_to_image(rect).unwrap();

    // let mut pixels = image.into_raw().clone();
    // pixels.truncate(pixels.len() / 4 * 4);
    // let bytes = image.to_rgba8().to_vec();

    // println!("{:?}", bytes);
    let handler = Handle::from_rgba(image.width(), image.height(), image.into_raw());
    // let handler = iced::widget::image::allocate(image);

    let message = allocate(handler).map(|e| Message::ImageAllocated(e.unwrap()));

    let _ = servo.state.rendering_context.present();
    // Ok(Bytes::from(image.to_vec()))
    // Ok(handler)
    // Ok(Bytes::from(pixels))
    Ok(message)
}
