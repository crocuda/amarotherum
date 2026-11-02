use super::Tab;
use iced_core::image::Allocation;

use crate::gui::state::{Message, Servo, State};

// Wgpu
use crate::gui::program::Program;
// use iced::Program;

use iced::widget::{Widget, bottom, button, column, container, row};
use iced::window;
use iced::{Length, Subscription};
use iced::{Settings, Task, Theme};
use iced_futures::backend::native::tokio::Executor as TokioExecutor;
use iced_wgpu::Renderer;

// Winit
use iced_winit::winit;

// Servo
use servo::{EventLoopWaker, WebView, WebViewId};

// Globals
use std::rc::Rc;
use std::sync::Arc;

// Error
use amarotherum_error::AmaroError;
use miette::Result;
use tracing::{debug, error, info, trace, warn};

pub struct App {
    state: State,
    tabs: Vec<Tab>,
    selected_tab: Option<i32>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: State::default(),
            tabs: vec![],
            selected_tab: None,
        }
    }
    pub fn run(mut self) -> anyhow::Result<()> {
        let (event_loop, runtime, proxy) = crate::gui::winit::run::create_runtime(&mut self)?;
        self.state.proxy = Some(proxy.clone());
        crate::gui::winit::run::run(self, event_loop, runtime, proxy)?;
        Ok(())
    }
}

pub trait ExtraMessage<Message> {
    // type Message;
    fn window_created(window: Arc<winit::window::Window>) -> Message;
}

impl ExtraMessage<Message> for App {
    // type Message = Message;
    fn window_created(window: Arc<winit::window::Window>) -> Message {
        Message::WindowCreated { window }
    }
}
impl ExtraMessage<Message> for &mut App {
    // type Message = Message;
    fn window_created(window: Arc<winit::window::Window>) -> Message {
        Message::WindowCreated { window }
    }
}

impl Program for App {
    type State = State;
    type Message = Message;
    type Theme = Theme;
    type Renderer = Renderer;
    type Executor = TokioExecutor;

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
    fn update(&mut self, _state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        debug!("Message={:#?}", message);
        // Update internal components
        match message.clone() {
            Message::WindowCreated { window } => {
                let servo = Rc::new(Servo::try_new(&window).unwrap());
                self.state.servo = Some(servo.clone());
                self.tabs
                    .push(Tab::try_new(Rc::new(self.state.clone())).unwrap())
            }
            Message::Tick => {
                if let Some(servo) = &self.state.servo {
                    servo.instance.spin_event_loop();
                }
            }
            _ => {}
        }
        // Pass messages to sub components
        let tasks: Vec<Task<Message>> = self
            .tabs
            .iter_mut()
            .map(|e| e.update(message.clone()))
            .collect();
        Task::batch(tasks)
    }

    #[tracing::instrument(skip_all)]
    fn view<'a>(
        &self,
        _state: &'a Self::State,
        _window: window::Id,
    ) -> impl Widget<Self::Message, Self::Theme, Self::Renderer> + 'a {
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
    /// Emit regular event to redraw window.
    /// 16ms -> 60fps
    /// 8ms -> 120fps
    fn subscription(&self, _state: &Self::State) -> Subscription<Message> {
        let event_logger = iced::event::listen_raw(|event, _status, _id| {
            debug!("{:#?}", event);
            None
        });
        let tick = if let Some(servo) = &self.state.servo {
            // iced::time::every(std::time::Duration::from_millis(8)).map(|_| Message::Tick)
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
            // iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };
        Subscription::batch([event_logger, tick])
    }
}

impl Program for &mut App {
    type State = State;
    type Message = Message;
    type Theme = Theme;
    type Renderer = Renderer;
    type Executor = TokioExecutor;

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
    fn update(&mut self, _state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        debug!("Message={:#?}", message);
        // Update internal components
        match message.clone() {
            Message::WindowCreated { window } => {
                let servo = Rc::new(Servo::try_new(&window).unwrap());
                self.state.servo = Some(servo.clone());
                self.tabs
                    .push(Tab::try_new(Rc::new(self.state.clone())).unwrap())
            }
            Message::Tick => {
                if let Some(servo) = &self.state.servo {
                    servo.instance.spin_event_loop();
                }
            }
            _ => {}
        }
        // Pass messages to sub components
        let tasks: Vec<Task<Message>> = self
            .tabs
            .iter_mut()
            .map(|e| e.update(message.clone()))
            .collect();
        Task::batch(tasks)
    }

    #[tracing::instrument(skip_all)]
    fn view<'a>(
        &self,
        _state: &'a Self::State,
        _window: window::Id,
    ) -> impl Widget<Self::Message, Self::Theme, Self::Renderer> + 'a {
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
    /// Emit regular event to redraw window.
    /// 16ms -> 60fps
    /// 8ms -> 120fps
    fn subscription(&self, _state: &Self::State) -> Subscription<Message> {
        let event_logger = iced::event::listen_raw(|event, _status, _id| {
            debug!("{:#?}", event);
            None
        });
        let tick = if let Some(servo) = &self.state.servo {
            // iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };
        Subscription::batch([event_logger, tick])
    }
}
