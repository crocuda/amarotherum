use crate::gui::components::Message;
use crate::gui::view::{Proxy, ServoState};

use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{ContentFit, Length};
use iced_wgpu::Renderer;
use iced_winit::core::{Color, Theme};

// Images
use iced::advanced::image::Id;
use iced::widget::image::{FilterMethod, Handle, Image};
use image::{DynamicImage, EncodableLayout, ImageReader, RgbaImage};

// Servo
use dpi::PhysicalSize;
use euclid::{Box2D, Point2D, Scale, Size2D};
use servo::{
    DevicePixel, Servo, ServoBuilder, WebView, WebViewBuilder, WebViewDelegate, WebViewId,
};
use servo::{OffscreenRenderingContext, RenderingContext};
use url::Url;

// Globals
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use tracing::{debug, error, info, trace, warn};
// Error
use amarotherum_error::AmaroError;
use miette::Result;

#[derive(Clone)]
pub struct Tab {
    state: Rc<ServoState>,
    webview: WebView,
    last_frame: Handle,
}

impl Tab {
    pub fn new(state: Rc<ServoState>, proxy: &Proxy) -> Self {
        let url = Url::parse("https://elrumordelaluz.github.io/reshake/").unwrap();
        // let url = Url::parse("https://demo.servo.org").expect("Guaranteed by argument");
        let webview = WebViewBuilder::new(&state.servo, state.rendering_context.clone())
            .url(url)
            .hidpi_scale_factor(Scale::new(state.window.clone().scale_factor() as f32))
            .delegate(Rc::new(proxy.clone()))
            .build();

        Self {
            state,
            webview,
            last_frame: Handle::from_bytes(""),
        }
    }
}

impl Tab {
    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, state: Rc<ServoState>) {
        debug!("update tab");
        if let Ok(image) = webview_to_image(self, state.clone()) {
            self.last_frame = image;
        }
        // DANGER: redraw requests shouldn't originated from here.
        let _ = state.window.request_redraw();
    }
    #[tracing::instrument(skip_all)]
    pub fn view(&self) -> impl Widget<Message, Theme, Renderer> + use<'_> {
        debug!("view tab");
        let image: Image<Handle> = Image::new(self.last_frame.clone())
            // let image: Image<Handle> = Image::new("/home/anon/Pictures/megatron_pp.png")
            .filter_method(FilterMethod::Nearest)
            .content_fit(ContentFit::Fill)
            .expand(true);
        //
        column![
            container("title"),
            container(image)
                .align_top(Length::Fill)
                .align_left(Length::Fill)
                .style(container::bordered_box),
        ]
    }
}

/// Turn the last frame of a servo webview into an image that can be printed by iced.
#[tracing::instrument(skip_all)]
pub fn webview_to_image(tab: &Tab, state: Rc<ServoState>) -> Result<Handle, AmaroError> {
    // debug!("LoadStatus={:#?}", tab.webview.load_status());
    let _ = state.rendering_context.prepare_for_rendering();
    tab.webview.paint();

    let _ = state.rendering_context.present();

    // Servo image
    let rect = state.rect();
    let image = DynamicImage::ImageRgba8(state.rendering_context.read_to_image(rect).unwrap());
    let w = image.width();
    let h = image.height();
    let bytes = image.to_rgba8().into_raw();

    // println!("{:?}", bytes);
    let handler = Handle::from_rgba(w, h, bytes);

    Ok(handler)
}
