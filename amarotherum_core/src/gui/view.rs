use iced::advanced::{Widget, image};
use iced::{
    Element, Fill, Renderer, Subscription, Task, Theme,
    widget::{Column, Container, Image, Row, Text, column, container, row, text, text_input},
};
use iced::{Program, Settings};
use iced_winit::futures::backend::native::thread_pool::Executor;
use tracing::{debug, info};

use dpi::PhysicalSize;
use euclid::{Box2D, Point2D, Scale};

use indexmap::IndexMap;
use raw_window_handle::{DisplayHandle, WaylandDisplayHandle, WaylandWindowHandle};
use servo::{
    DevicePixel, RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebView,
    WebViewBuilder, WebViewDelegate,
};
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use url::Url;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use amarotherum_error::AmaroError;
use miette::Result;

#[derive(Debug, Clone)]
enum Message {
    // WebView(Action),
    ViewCreated,
    Id(u64),
}

struct App {
    window: Option<Window>,
    servo: Servo,
    rendering_context: Rc<dyn RenderingContext>,
    webviews: IndexMap<String, WebView>,
}
impl Default for App {
    fn default() -> Self {
        let servo = ServoBuilder::default().build();

        let width = 500;
        let height = 500;
        let initial_size = PhysicalSize::new(width, height);
        let rendering_context = SoftwareRenderingContext::new(initial_size).unwrap();

        Self {
            window: None,
            servo,
            rendering_context: Rc::new(rendering_context),
            webviews: IndexMap::new(),
        }
    }
}
impl servo::WebViewDelegate for App {
    fn notify_new_frame_ready(&self, _: WebView) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[derive(Default, Debug, Clone)]
struct State {
    running: bool,
}

impl App {
    fn new() -> Self {
        Self::default()
    }
    fn rect(&self) -> Box2D<i32, DevicePixel> {
        let size = self.rendering_context.size();
        println!("Size={:#?}", size);
        Box2D::new(
            Point2D::new(0, 0),
            Point2D::new(size.width as i32, size.height as i32),
        )
    }

    fn update(&mut self, message: Message) {
        // fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Message::WebView(action) => self.webview.update(action),
            Message::ViewCreated => {
                //     self.ready = true;
                //     self.webview.update(Action::ChangeView(0))
            }
            _ => {}
        }
    }

    fn view(&self) -> impl Widget<Message, Theme, Renderer> {
        let windows = container("windows")
            .padding(5)
            .align_top(Fill)
            .align_left(Fill)
            .style(container::bordered_box);
        let statusline = container("statusline");
        let commandline = container("commandline");

        // let mut wid = String::new();
        // if let Some(window) = &self.window() {
        // let raw_id: Task<u64> = iced::window::raw_id::<Message>(self.window);

        // let event_loop = EventLoop::new().unwrap();
        // event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        // event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

        let webview = WebViewBuilder::new(&self.servo, self.rendering_context.clone()).build();
        webview.load(Url::parse("https://duckduckgo.com").unwrap());
        webview.paint();

        // self.rendering_context.present();

        // thread::sleep(Duration::from_millis(10000));

        println!("LoadStatus={:#?}", webview.load_status());

        let image_buffer = self.rendering_context.read_to_image(self.rect()).unwrap();
        // println!("image_buffer={:?}", &image_buffer);

        let rendered_url: Image =
            iced::widget::image(image::Handle::from_bytes(image_buffer.to_vec()));

        column![windows, rendered_url, statusline, commandline]
    }

    // fn subscription(&self) -> Subscription<Message> {}
}
// impl Program for App {
//     type State = State;
//     type Message = Message;
//     type Theme = Theme;
//     type Renderer = Renderer;
//     type Executor = Executor;
//
//     fn name() -> &'static str {
//         "amarotherum"
//     }
//     fn settings(&self) -> iced::Settings {
//         Settings::default()
//     }
//     fn window(&self) -> Option<iced::window::Settings> {
//         Some(iced::window::Settings::default())
//     }
//     fn boot(&self) -> (Self::State, Task<Self::Message>) {
//         (State::default(), Task::none())
//     }
//     fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
//         Task::none()
//     }
//     fn view<'a>(
//         &self,
//         state: &'a Self::State,
//         window: iced_winit::core::window::Id,
//     ) -> impl Widget<Self::Message, Self::Theme, Self::Renderer> + 'a {
//         let windows = container("windows")
//             .padding(5)
//             .align_top(Fill)
//             .align_left(Fill)
//             .style(container::bordered_box);
//         let statusline = container("statusline");
//         let commandline = container("commandline");
//
//         // let mut wid = String::new();
//         // if let Some(window) = &self.window() {
//         // let raw_id: Task<u64> = iced::window::raw_id::<Message>(window);
//
//         // let event_loop = EventLoop::new().unwrap();
//         // event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
//         // event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
//
//         let webview = WebViewBuilder::new(&self.servo, self.rendering_context.clone())
//             .url(Url::parse("https://duckduckgo.com").unwrap());
//
//         let image_buffer = self.rendering_context.read_to_image(self.rect()).unwrap();
//         self.rendering_context()
//
//         println!("image_buffer={:?}", &image_buffer);
//         let rendered_url: Image =
//             iced::widget::image(image::Handle::from_bytes(image_buffer.to_vec()));
//
//         column![windows, rendered_url, statusline, commandline]
//     }
// }

#[derive(Default)]
struct ServoHeadless {
    window: Option<Window>,
}

impl ApplicationHandler for ServoHeadless {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("amarotherum");
        self.window = Some(event_loop.create_window(window_attributes).unwrap());
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        info!("user_event")
        // self.update(message);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Focused(bool) => {}
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

pub fn main() -> Result<(), AmaroError> {
    let event_loop = EventLoop::new().unwrap();
    // event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    // let mut app = App::new();
    let mut app = iced::application(App::new, App::update, App::view)
        .theme(Theme::Dark)
        .centered()
        .title("amarotherum");

    iced_winit::run(app).unwrap();
    // let _ = event_loop.run_app(&mut app);

    Ok(())
}

pub fn create_servo() -> Result<()> {
    let servo = ServoBuilder::default().build();

    // let display_handle = WaylandDisplayHandle::new(display);
    // let window_handle = WaylandWindowHandle::new(surface);
    //
    // let ctx = WindowRenderingContext::new();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window() -> Result<()> {
        // main().unwrap();
        Ok(())
    }
}
