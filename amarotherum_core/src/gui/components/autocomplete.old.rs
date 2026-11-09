use iced::color::Color;
use iced::font;
use iced::overlay::menu::Catalog;
use iced::widget::{
    Float, Id, Widget, bottom, button, column, container, overlay::menu, row, slider, text,
    text_input,
};
use iced::{Background, ContentFit, Length, Task, Theme};
use iced::{Size, Vector};
use iced_wgpu::Renderer;

// use iced::widget::PickList
use super::_autocomplete::PickList;

use crate::gui::state::{Action, Command, Message, Mode, State};

use strum::IntoEnumIterator;

use std::rc::Rc;

// Error
use miette::{Error, Result, miette};

#[derive(Clone)]
pub struct Autocomplete {
    pub state: Rc<State>,
    // pub menu_state: menu::State,
    pub actions: Vec<Action>,
    pub selected: Option<Action>,

    // Index of overed item.
    hovered_option: Option<usize>,
    // pub theme: <Theme as menu::Catalog>::Class<'a>,
    // pub theme: Theme,
    // pub theme: Box<Theme>,
    // pub theme: Theme,
}

impl Autocomplete
where
    Action: Clone + ToString + IntoEnumIterator,
{
    pub fn try_new(state: Rc<State>) -> Result<Self> {
        let menu_state = menu::State::new();
        let actions: Vec<Action> = Action::iter().collect();
        // let theme = <Theme as Catalog>::default();
        let theme = Theme::default();

        let hovered_option = Some(0);

        // on_option_hovered: dyn Fn(T) -> Message,
        Ok(Self {
            state,
            // menu_state,
            actions,
            selected: None,
            hovered_option,
            // theme,
            // to_string: &|e: &Action| -> String {
            //     return e.to_string();
            // },
        })
    }
    // pub fn menu(&mut self) -> menu::Menu<'_, '_, Action, Message, Theme> {
    pub fn menu<'a>(&'a self) -> impl Widget<Message, Theme, Renderer> + 'a {
        let on_selected = |e: Action| -> Message {
            return Message::Null;
        };
        let to_string = &|e: &Action| -> String {
            return e.to_string();
        };
        let theme = <Theme as Catalog>::default();

        Float::new(
            container(
                PickList::new(
                    self.selected.clone(),
                    self.actions.clone(),
                    Action::to_string,
                )
                .on_select(on_selected)
                .placeholder("coucou"),
            )
            .height(300),
        )
        .translate(|_, _| Vector::new(-100.0, -100.0))
    }
}

impl Autocomplete {
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&'a self) -> impl Widget<Message, Theme, Renderer> + 'a {
        let target_height = 12.0;
        let window = Size {
            width: 10.0,
            height: 10.0,
        };
        let position = iced::Point::new(10.0, 10.0);

        self.menu()
        // self.menu()
        //     .overlay(renderer, position, window, target_height)
    }
}
