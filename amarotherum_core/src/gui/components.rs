use crate::gui::tabs::{self, Tab};
use crate::gui::view::{Proxy, ServoState};

use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{ContentFit, Length, Subscription};
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

#[derive(Clone, Debug)]
pub enum Message {
    WebViewFrame(WebViewId),
    Keyboard,
}

pub struct App {
    servo_state: Rc<ServoState>,
    proxy: Proxy,
    // Tab list
    tabs: Vec<Tab>,
    // App state
    selected_tab: Option<i32>,
}

impl App {
    pub fn new(servo_state: Rc<ServoState>, proxy: &Proxy) -> Self {
        let default_tab = Tab::new(servo_state.clone(), proxy);
        Self {
            servo_state,
            proxy: proxy.clone(),
            tabs: vec![default_tab],
            selected_tab: None,
        }
    }
}

impl App {
    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, message: Message) {
        debug!("update app");

        // Update internal components
        match message {
            Message::WebViewFrame(web) => {
                self.tabs
                    .iter_mut()
                    .for_each(|e| e.update(self.servo_state.clone()));
            }
            _ => {}
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn view(&self) -> impl Widget<Message, Theme, Renderer> {
        debug!("view app");
        if let Some(tab) = self.tabs.last() {
            return column![
                container("buffline"),
                container(tab.view())
                    .align_top(Length::Fill)
                    .align_left(Length::Fill)
                    .style(container::bordered_box),
                container("statusline"),
                container("commandline")
            ];
        } else {
            return column![
                container("buffline"),
                container("empty")
                    .align_top(Length::Fill)
                    .align_left(Length::Fill)
                    .style(container::bordered_box),
                container("statusline"),
                container("commandline"),
            ];
        }
    }
}
