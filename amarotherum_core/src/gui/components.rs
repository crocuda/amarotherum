use crate::gui::draw;
use crate::gui::view::{Message, ServoState};

use iced::advanced::image::Id;
use iced::widget::image::{FilterMethod, Handle, Image};
use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{ContentFit, Length};
use iced_wgpu::Renderer;
use iced_winit::core::{Color, Theme};

use image::{DynamicImage, EncodableLayout, ImageReader, RgbaImage};

use std::rc::Rc;

use tracing::{debug, error, info, trace, warn};

// Error
use amarotherum_error::AmaroError;
use miette::Result;

pub struct App {
    servo_state: Rc<ServoState>,
    last_frame: Handle,
}

impl App {
    pub fn new(servo_state: Rc<ServoState>) -> Self {
        Self {
            servo_state,
            last_frame: Handle::from_bytes(""),
        }
    }
}

impl App {
    pub fn update(&mut self, message: Message) {
        info!("update App");
        match message {
            Message::WebViewFrame => {
                self.last_frame = draw::webview_to_image(&self.servo_state).unwrap();
            }
            _ => {}
        }
    }

    pub fn view(&self) -> impl Widget<Message, Theme, Renderer> {
        debug!("draw App view");
        let handler = self.last_frame.clone();
        debug!("{:#?}", handler);

        let image: Image<Handle> = Image::new(self.last_frame.clone())
            // let image: Image<Handle> = Image::new("/home/anon/Pictures/megatron_pp.png")
            .filter_method(FilterMethod::Nearest)
            .content_fit(ContentFit::Fill)
            .expand(true);

        column![
            container("buffline"),
            container(image)
                .align_top(Length::Fill)
                .align_left(Length::Fill)
                .style(container::bordered_box),
            container("statusline"),
            container("commandline"),
            button("send").on_press(Message::WebViewFrame)
        ]
    }
}
