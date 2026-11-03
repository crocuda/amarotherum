use core::fmt;

use crate::gui::state::{Message, Servo, State};
use iced::color::Color;
use iced::font;
use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{Background, ContentFit, Length, Task, Theme};
use iced_wgpu::Renderer;

use core::str::FromStr;

use convert_case::{Case, Casing};

use std::rc::Rc;

#[derive(Default, Clone, Debug)]
pub struct CommandLine {
    pub input: Rc<String>,
}

impl CommandLine {
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&self) -> impl Widget<Message, Theme, Renderer> + 'a {
        let input = String::new();
        container(row![
            text_input("", input)
                .font(
                    font::Font::new("JetBrainsMono")
                        .weight(font::Weight::Medium)
                        .style(font::Style::Normal),
                )
                .size(14)
        ])
        .align_left(Length::Fill)
        .style(|_| container::background(Background::Color(Color::from_str("#282c34").unwrap())))
    }
}
