use crate::gui::state::{Message, Servo, State};

use iced::widget::{Widget, bottom, button, column, container, row, slider, text, text_input};
use iced::{ContentFit, Length, Task};
use iced_wgpu::Renderer;
use iced_winit::core::{Color, Theme};

// Images
// use iced::advanced::image::{Image, allocate};
use iced::widget::image::{Allocation, FilterMethod, Handle};
use iced::widget::image::{Image, allocate};
use iced_core::Bytes;
use iced_core::image::Handle::Rgba;
use image::DynamicImage;

// Servo
use euclid::Scale;
use futures::channel::oneshot;
use servo::{JSValue, JavaScriptEvaluationError};
use servo::{OffscreenRenderingContext, RenderingContext};
use servo::{WebView, WebViewBuilder, WebViewId};

use url::Url;

use std::fs;
// Globals
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use tracing::{debug, error, info, trace, warn};
// Error
use amarotherum_error::{AmaroError, LibError};
use miette::{Error, Result, miette};

#[derive(Clone)]
pub struct Tab {
    state: Rc<State>,
    webview: WebView,
    // last_frame: Arc<RwLock<Handle>>,
    last_frame: Option<Allocation>,

    // Js
    title: String,
}

impl Tab {
    pub fn try_new(state: Rc<State>) -> Result<Self> {
        let url = Url::parse("https://elrumordelaluz.github.io/reshake/").unwrap();
        // let url = Url::parse("https://demo.servo.org").expect("Guaranteed by argument");
        let webview = WebViewBuilder::new(
            &state.servo.instance,
            state.servo.state.rendering_context.clone(),
        )
        .url(url)
        .hidpi_scale_factor(Scale::new(
            state.servo.state.window.clone().scale_factor() as f32
        ))
        .delegate(state.clone())
        .build();
        // TEST: Load local image.
        // let bytes = fs::read("/home/anon/Pictures/megatron_pp.png").unwrap();
        Ok(Self {
            state: state,
            webview,
            // last_frame: Arc::new(RwLock::new(Handle::from_bytes(""))),
            // last_frame: Handle::from_bytes(bytes),
            last_frame: None,
            //
            title: "New tab".to_owned(),
        })
    }
}

impl Tab {
    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, message: Message) -> iced_runtime::Task<Message> {
        match message {
            Message::DefaultJsEvaluate => {
                let mut title: String = self.webview.page_title().unwrap();
                if let Some(url) = self.webview.url() {
                    title = format!("{} - {}", title, url);
                }
                self.title = title;
                Task::none()

                // self
                // .evaluate_javascript_task("document.title")
                // .map(Message::TitleLoaded),
            }

            Message::TitleLoaded(Ok(JSValue::String(title))) => {
                self.title = title;
                Task::none()
            }
            Message::WebViewHasNewFrame(webview_id) => {
                if webview_id != self.webview.id() {
                    return Task::none();
                }
                if let Ok(image) = webview_to_image(self, self.state.servo.clone()) {
                    image
                } else {
                    Task::none()
                }
            }
            Message::ImageAllocated(allocation) => {
                self.last_frame = Some(allocation);
                Task::none()
            }
            Message::ServoNeedsResize(size) => {
                self.webview.resize(size);
                Task::none()
            }
            _ => Task::none(),
        }
    }
    #[tracing::instrument(skip_all)]
    pub fn view<'a>(&self) -> impl Widget<Message, Theme, Renderer> + 'a {
        // TEST: Load local image.
        // let image: Image<Handle> = Image::new("/home/anon/Pictures/megatron_pp.png")
        if let Some(allocation) = self.last_frame.clone() {
            let image: Image<Handle> = Image::new(allocation.handle())
                .filter_method(FilterMethod::Nearest)
                .content_fit(ContentFit::Fill)
                .expand(true);
            column![
                text(self.title.clone()),
                container(image)
                    .align_top(Length::Fill)
                    .align_left(Length::Fill)
                    .style(container::bordered_box),
            ]
        } else {
            column![
                container("empty")
                    .align_top(Length::Fill)
                    .align_left(Length::Fill)
                    .style(container::bordered_box),
            ]
        }
    }
}

impl Tab {
    /// Evaluate a JavaScript snippet in the webview and deliver the
    /// result through a callback. Servo runs the script asynchronously
    /// (on the next `spin_event_loop` tick that pumps the script
    /// thread) and invokes `callback` on the main thread when the
    /// result is ready or an error occurs.
    ///
    /// Prefer [`evaluate_javascript_task`](Self::evaluate_javascript_task)
    /// for iced applications — it hands you back a `Task<Message>` you
    /// can return from `update()`, which is the ergonomic fit for the
    /// Elm-style model. Use this lower-level callback form when you
    /// need to drive side effects without going through the Message
    /// enum.
    pub fn evaluate_javascript(
        &self,
        script: impl ToString,
        callback: impl FnOnce(Result<JSValue, JavaScriptEvaluationError>) + 'static,
    ) {
        self.webview.evaluate_javascript(script, callback);
    }

    /// Evaluate a JavaScript snippet and wrap the asynchronous result
    /// in an iced [`Task`]. Typical usage in an app's `update`:
    ///
    /// ```ignore
    /// Message::RunScript => controller
    ///     .evaluate_javascript_task("document.title")
    ///     .map(Message::ScriptResult),
    /// ```
    ///
    /// The task resolves once Servo has finished executing the script.
    /// If the oneshot channel bridging Servo's callback to the task is
    /// dropped (e.g. the controller is destroyed mid-eval), the task
    /// resolves to `Err(JavaScriptEvaluationError::InternalError)` — we
    /// reuse that variant rather than inventing a new one because an
    /// embedder drop genuinely looks like "Servo couldn't complete the
    /// evaluation" from the caller's point of view.
    pub fn evaluate_javascript_task(
        &self,
        script: impl ToString,
    ) -> Task<Result<JSValue, JavaScriptEvaluationError>> {
        let (tx, rx) = oneshot::channel();
        self.webview.evaluate_javascript(script, move |result| {
            let _ = tx.send(result);
        });
        Task::future(async move {
            rx.await
                .unwrap_or(Err(JavaScriptEvaluationError::InternalError))
        })
    }
}

/// Turn the last frame of a servo webview into an image that can be printed by iced.
#[tracing::instrument(skip_all)]
// pub fn webview_to_image(tab: &Tab, servo: Rc<Servo>) -> Result<Handle, AmaroError> {
// pub fn webview_to_image(tab: &Tab, servo: Rc<Servo>) -> Result<Bytes, AmaroError> {
pub fn webview_to_image(tab: &Tab, servo: Rc<Servo>) -> Result<Task<Message>> {
    // debug!("LoadStatus={:#?}", tab.webview.load_status());
    // let _ = servo.state.rendering_context.make_current();
    // let _ = servo.state.rendering_context.prepare_for_rendering();
    tab.webview.paint();

    // Servo image
    let rect = servo.state.rect();
    // let image =
    //     DynamicImage::ImageRgba8(servo.state.rendering_context.read_to_image(rect).unwrap());
    //
    let image = servo.state.rendering_context.read_to_image(rect).unwrap();

    // let mut pixels = image.into_raw().clone();
    // pixels.truncate(pixels.len() / 4 * 4);
    // let bytes = image.to_rgba8().to_vec();

    // println!("{:?}", bytes);
    let handler = Handle::from_rgba(image.width(), image.height(), image.into_raw());
    // let handler = iced::widget::image::allocate(image);

    let message = allocate(handler).map(|e| Message::ImageAllocated(e.unwrap()));

    // let _ = servo.state.rendering_context.present();
    // Ok(Bytes::from(image.to_vec()))
    // Ok(handler)
    // Ok(Bytes::from(pixels))
    Ok(message)
}
