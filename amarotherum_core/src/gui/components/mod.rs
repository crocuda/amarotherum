mod tabs;
use tabs::Tab;

use crate::gui::state::{Message, Proxy, ServoState};

// Wgpu
use iced::widget::{Widget, bottom, button, column, container, row};
use iced::window;
use iced::{Length, Subscription};
use iced::{Program, Settings, Task, Theme};
use iced_futures::backend::native::tokio::Executor;
use iced_wgpu::Renderer;

// Winit
use iced_winit::winit;
use winit::window::Window;

// Globals
use std::rc::Rc;
use std::sync::Arc;

// Error
use amarotherum_error::AmaroError;
use miette::Result;
use tracing::{debug, error, info, trace, warn};

/// App state
#[derive(Default)]
pub struct State {
    // Tab list
    tabs: Vec<Tab>,
    selected_tab: Option<i32>,
    // servo_state: Rc<ServoState>,
}
pub struct App {
    // window: Arc<Window>,
    //
    // proxy: Proxy,
}

impl App {
    pub fn new() -> Self {
        // pub fn new(servo_state: Rc<ServoState>, proxy: &Proxy) -> Self {
        // let default_tab = Tab::new(servo_state.clone(), proxy);
        Self {
            // servo_state,
            // proxy: proxy.clone(),
            // tabs: vec![default_tab],
            // selected_tab: None,
        }
    }
}

impl Program for App {
    type State = State;
    type Message = Message;
    type Theme = Theme;
    type Renderer = Renderer;
    type Executor = Executor;

    fn name() -> &'static str {
        "amarotherum"
    }
    fn settings(&self) -> Settings {
        Settings::default()
    }
    fn window(&self) -> Option<window::Settings> {
        Some(window::Settings::default())
    }
    fn boot(&self) -> (Self::State, Task<Self::Message>) {
        (Self::State::default(), Task::none())
    }
    #[tracing::instrument(skip_all)]
    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        // debug!("update app");
        debug!("{:#?}", message);
        // Update internal components
        // match message {
        //     Message::WebViewFrame(id) => {
        //         self.tabs
        //             .iter_mut()
        //             .for_each(|e| e.update(self.servo_state.clone()));
        //     }
        //     _ => {}
        // }
        Task::none()
    }

    #[tracing::instrument(skip_all)]
    fn view<'a>(
        &self,
        _state: &'a Self::State,
        window: window::Id,
    ) -> impl Widget<Self::Message, Self::Theme, Self::Renderer> + 'a {
        debug!("view app");
        // // if let Some(tab) = self.tabs.last() {
        //     return column![
        //         container("buffline")
        //             // container(tab.view())
        //             .align_top(Length::Fill)
        //             .align_left(Length::Fill)
        //             .style(container::bordered_box),
        //         container("statusline"),
        //         container("commandline")
        //     ];
        // } else {
        return column![
            container("buffline"),
            container("empty")
                .align_top(Length::Fill)
                .align_left(Length::Fill)
                .style(container::bordered_box),
            container("statusline"),
            container("commandline"),
        ];
        // }
    }
    fn subscription(&self, _state: &Self::State) -> Subscription<Message> {
        // let winit_logger = winit::dpi::;
        let event_logger = iced::event::listen_raw(|event, status, id| {
            debug!("{:#?}", event);
            None
        });
        Subscription::batch([
            event_logger,
            // 16ms -> 60fps
            // 8ms -> 120fps
            // iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
            // self.runtime.subscription().map(|_| Message::Tick),
        ])
    }
}
