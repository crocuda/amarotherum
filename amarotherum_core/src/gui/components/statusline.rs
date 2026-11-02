use core::fmt;

use crate::gui::state::{Message, Servo, State};
use iced::color;
use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{Background, ContentFit, Length, Task};
use iced_wgpu::Renderer;

use core::str::FromStr;
use iced_winit::core::{Color, Theme};

use convert_case::{Case, Casing};

// Error
use amarotherum_error::{AmaroError, LibError};
use miette::{Error, Result, miette};

#[derive(Default, Clone, Debug)]
pub enum Mode {
    #[default]
    Normal,
    Visual,
    Follow,
    Command,
}
impl ToString for Mode {
    fn to_string(&self) -> String {
        let res: String = format!("{:?}", self);
        res.to_case(Case::Pascal)
    }
}
impl Mode {
    pub fn to_widget<'a>(&self) -> impl Widget<Message, Theme, Renderer> + 'a {
        let res = container(text(self.to_string()));
        let container = match &self {
            // Normal -> Blue
            Mode::Normal => res.style(|_| {
                container::background(Background::Color(Color::from_str("#48a6e6").unwrap()))
            }),
            // Visual -> Bright blue
            Mode::Visual => res.style(|_| {
                container::background(Background::Color(Color::from_str("#48a6e6").unwrap()))
            }),
            // Follow -> ?
            Mode::Follow => res.style(|_| {
                container::background(Background::Color(Color::from_str("#48a6e6").unwrap()))
            }),
            // Command -> Green
            Mode::Command => res.style(|_| {
                container::background(Background::Color(Color::from_str("#48a6e6").unwrap()))
            }),
            // Insert: Pink
        };
        container
    }
}

#[derive(Default, Clone, Debug)]
pub struct StatusLine {
    mode: Mode,
}
impl StatusLine {
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&self) -> impl Widget<Message, Theme, Renderer> + 'a {
        row![self.mode.to_widget()]
    }
}
