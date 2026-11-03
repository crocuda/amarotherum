use iced::color::Color;
use iced::font;
use iced::widget::{Id, Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{Background, ContentFit, Length, Task, Theme};
use iced_wgpu::Renderer;

use core::str::FromStr;
use std::rc::Rc;

use crate::gui::state::{Action, Command, Message, Mode};

#[derive(Clone, Debug)]
pub struct CommandLine {
    // The focusable text input id.
    pub id: Id,
    pub input: Rc<String>,
}
impl CommandLine {
    pub fn new() -> Self {
        Self {
            id: Id::unique(),
            input: Rc::new(String::new()),
        }
    }
}

impl CommandLine {
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&self) -> impl Widget<Message, Theme, Renderer> + 'a {
        let text_input = text_input("", String::new())
            .id(self.id.clone())
            .font(
                font::Font::new("JetBrainsMono")
                    .weight(font::Weight::Medium)
                    .style(font::Style::Normal),
            )
            .size(14);
        container(row![text_input])
            .align_left(Length::Fill)
            .style(|_| {
                container::background(Background::Color(Color::from_str("#282c34").unwrap()))
            })
    }

    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FocusCommandLine(mode) => match mode {
                Mode::Command => Task::none(),
                _ => Task::none(),
            },
            _ => Task::none(),
        }
    }
}
