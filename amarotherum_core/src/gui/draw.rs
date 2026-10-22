use super::view::{ServoState, State};
use iced::widget::image::{Handle, Image};
use servo::RenderingContext;

// Error
use amarotherum_error::AmaroError;
use miette::Result;

pub fn webview_to_image(state: &ServoState) -> Result<Image, AmaroError> {
    let webviews = state.webviews.borrow();
    let (_id, webview) = webviews.last().unwrap();

    // debug!("LoadStatus={:#?}", webview.load_status());
    webview.paint();
    // state.rendering_context.present();

    // Servo image
    let rect = state.rect();
    let image = state.rendering_context.read_to_image(rect.clone()).unwrap();

    // Iced image
    let image = Image::new(Handle::from_rgba(
        image.width(),
        image.height(),
        image.into_raw(),
    ));

    Ok(image)
}
