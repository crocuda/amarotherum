use super::Tab;
use iced_core::image::Allocation;

use crate::config::Config;
use crate::gui::components::{CommandLine, StatusLine};
use crate::gui::state::{Message, Mode, Servo, State, StateType};

// Wgpu
use crate::gui::program::Program;
// use iced::Program;

use iced::widget::{Id, Widget, bottom, button, column, container, row, text};
use iced::{Length, Subscription};
use iced::{Settings, Task, Theme};
use iced_core::keyboard::{self, Key, key::Named};
use iced_futures::backend::native::tokio::Executor as TokioExecutor;
use iced_wgpu::Renderer;

use iced::window;
use iced_core::event::Event;

// Winit
use iced_winit::winit;

// Servo
use dpi::PhysicalSize;
use servo::RenderingContext;
use servo::{EventLoopWaker, WebView, WebViewId};

// Globals
use std::rc::Rc;
use std::sync::Arc;

// Error
use amarotherum_error::AmaroError;
use miette::Result;
use tracing::{debug, error, info, trace, warn};

pub struct App {
    state: StateType,

    tabs: Vec<Tab>,
    selected_tab: Option<i32>,

    statusline: StatusLine,
    commandline: Option<CommandLine>,
}
impl App {
    pub fn run() -> anyhow::Result<()> {
        let (event_loop, runtime, proxy) = crate::gui::winit::run::create_runtime()?;
        let app = Self {
            state: StateType::Init(proxy.clone()),
            tabs: Vec::new(),
            selected_tab: None,
            statusline: StatusLine::default(),
            commandline: None,
        };
        crate::gui::winit::run::run(app, event_loop, runtime, proxy)?;
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
    type State = StateType;
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
        // debug!("Message={:#?}", message);
        if let StateType::Null = self.state {
            return Task::none();
        }
        // Update internal components
        else if let StateType::Init(proxy) = &self.state {
            match message.clone() {
                Message::WindowCreated { window } => {
                    let config = Config::get().unwrap();
                    let servo = Rc::new(Servo::try_new(&window).unwrap());
                    let inner_state = State {
                        config,
                        servo,
                        proxy: proxy.clone(),
                    };
                    let ready_state = StateType::Ready(inner_state.clone());
                    self.state = ready_state;

                    let inner_state = Rc::new(inner_state);
                    self.commandline = Some(CommandLine::new(inner_state.clone()));
                    self.tabs.push(Tab::try_new(inner_state.clone()).unwrap());

                    return Task::none();
                }
                _ => return Task::none(),
            }
        } else if let StateType::Ready(state) = &self.state {
            match message.clone() {
                Message::Keyboard(event) => {
                    return self.handle_keyboard(event).unwrap();
                }
                Message::Tick => {
                    state.servo.instance.spin_event_loop();
                    // Task::none()
                }
                Message::ServoNeedsResize(size) => {
                    // state.servo.state.rendering_context.resize(size);
                    // Task::none()
                }
                _ => {
                    // Task::none()
                }
            }
        } else {
            // Task::none()
        }

        // Pass messages to sub components
        let mut tasks: Vec<Task<Message>> = vec![];

        let stline_task = self.statusline.update(message.clone());
        tasks.push(stline_task);

        if let Some(ref mut commandline) = self.commandline {
            let cmdline_task = commandline.update(message.clone());
            tasks.push(cmdline_task);
        }
        let tab_tasks: Vec<Task<Message>> = self
            .tabs
            .iter_mut()
            .map(|e| e.update(message.clone()))
            .collect();
        tasks.extend(tab_tasks);

        Task::batch(tasks)
    }

    #[tracing::instrument(skip_all)]
    fn view<'a>(
        &'a self,
        state: &'a Self::State,
        _window: window::Id,
    ) -> impl Widget<Self::Message, Self::Theme, Self::Renderer> + 'a {
        if let Some(tab) = self.tabs.last() {
            let tab_view = container(tab.view())
                .align_top(Length::Fill)
                .align_left(Length::Fill)
                .style(container::bordered_box);
            if let Some(ref cmd) = self.commandline {
                return column![tab_view, self.statusline.view(), cmd.view(),];
            } else {
                return column![tab_view, self.statusline.view()];
            }
        } else {
            return column![self.statusline.view()];
        }
    }
    /// Emit regular event to redraw window.
    /// 16ms -> 60fps
    /// 8ms -> 120fps
    fn subscription(&self, _state: &Self::State) -> Subscription<Message> {
        let StateType::Ready(state) = &self.state else {
            return Subscription::none();
        };
        let event_logger: Subscription<Message> = iced::event::listen_raw(|event, _status, _id| {
            // debug!("{:#?}", event);
            match event {
                Event::Keyboard(event) => Some(Message::Keyboard(event)),
                Event::Window(event) => match event {
                    window::Event::Resized(size) => {
                        let size = PhysicalSize::new(size.width as u32, size.height as u32);
                        Some(Message::ServoNeedsResize(size))
                    }
                    _ => None,
                },
                _ => None,
            }
        });
        // let tick = iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick);
        let tick = iced::time::every(std::time::Duration::from_millis(1000)).map(|_| Message::Tick);
        let default_js = iced::time::every(std::time::Duration::from_millis(1000))
            .map(|_| Message::DefaultJsEvaluate);

        Subscription::batch([event_logger, tick, default_js])
    }
}

impl App {
    pub fn handle_keyboard(&mut self, event: keyboard::Event) -> anyhow::Result<Task<Message>> {
        if let Some(ref mut cmd) = self.commandline {
            let task = match event {
                // keyboard::Event::KeyPressed(key) => match key {
                keyboard::Event::KeyPressed {
                    modified_key: key, ..
                } => match key {
                    Key::Character(c) => match c.as_str() {
                        ":" => {
                            cmd.input = c.to_string();
                            let focus_input = iced::widget::operation::focus::<Id>(cmd.id.clone())
                                .map(|_| Message::Null);
                            let cursor_to_end =
                                iced::widget::operation::text_input::move_cursor_to_end::<Id>(
                                    cmd.id.clone(),
                                )
                                .map(|_| Message::Null);
                            let change_mode = Task::done(Message::FocusCommandLine(Mode::Command));
                            Task::batch(vec![change_mode, cursor_to_end, focus_input])
                        }
                        // "/" => Task::done(Message::FocusCommandLine(Mode::Command)),
                        _ => Task::none(),
                    },
                    Key::Named(name) => match name {
                        Named::Escape => {
                            cmd.input = "".to_string();
                            let change_mode = Task::done(Message::FocusCommandLine(Mode::Normal));

                            Task::batch(vec![change_mode])
                        }
                        _ => Task::none(),
                    },
                    _ => Task::none(),
                },
                _ => Task::none(),
            };
            return Ok(task);
        } else {
            return Ok(Task::none());
        }
    }
}
