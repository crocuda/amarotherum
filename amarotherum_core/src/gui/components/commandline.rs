use iced::color::Color;
use iced::font;
use iced::widget::{
    Float, Id, Widget, bottom, button, column, container, overlay::menu, row, slider, text,
    text_input,
};
use iced::{Background, ContentFit, Length, Task, Theme};
use iced::{Border, border::Radius};
use iced_graphics::text::cosmic_text::skrifa::outline::autohint;
use iced_wgpu::Renderer;

use core::str::FromStr;
use std::rc::Rc;

use super::autocomplete::Autocomplete;
use crate::gui::state::{Action, Command, Message, Mode, State};

#[derive(Clone)]
pub struct CommandLine {
    // The focusable text input id.
    pub id: Id,
    pub input: String,
    pub state: Rc<State>,
    pub autocomplete: Autocomplete,
}
impl CommandLine {
    pub fn new(state: Rc<State>) -> Self {
        Self {
            id: Id::unique(),
            input: String::new(),
            autocomplete: Autocomplete::try_new(state.clone()).unwrap(),
            state,
        }
    }
}

impl CommandLine {
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&'a self) -> impl Widget<Message, Theme, Renderer> + 'a {
        let text_input = text_input("", &self.input)
            .id(self.id.clone())
            .on_input(|s| Message::TypingCommand(s))
            .on_submit(Message::SubmitCommand)
            .font(
                font::Font::new("JetBrainsMono")
                    .weight(font::Weight::Medium)
                    .style(font::Style::Normal),
            )
            .size(14)
            .style(|_, _| iced::widget::text_input::Style {
                selection: Color::from_str("#282c34").unwrap(),
                background: Background::Color(Color::from_str("#282c34").unwrap()),
                border: Border {
                    color: Color::from_str("#282c34").unwrap(),
                    radius: Radius::default(),
                    width: 0_f32,
                },
                placeholder: Color::from_str("#282c34").unwrap(),
                value: Color::from_str("#a7aebb").unwrap(),
            });

        // let autocomplete = Autocomplete::try_new(self.state.clone()).unwrap();
        // let autocomplete = Menu::new();
        // container(text("float")))
        // .translate(|_, _| iced::Vector { x: -10.0, y: -10.0 });

        container(row![text_input, self.autocomplete.view()])
            // container(row![text_input])
            .align_left(Length::Fill)
            .style(|_| {
                container::background(Background::Color(Color::from_str("#282c34").unwrap()))
            })
            .height(30)
    }

    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TypingCommand(s) => {
                self.input = s;
                Task::none()
            }
            Message::FocusCommandLine(mode) => match mode {
                Mode::Command => {
                    // Show command suggestion list
                    Task::none()
                }
                _ => Task::none(),
            },
            _ => Task::none(),
        }
    }
}
