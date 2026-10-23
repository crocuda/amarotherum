use std::fs;

use iced::ContentFit;
use iced::advanced::image::{FilterMethod, Handle, Image};

use super::view::{ServoState, State};
use image::{DynamicImage, EncodableLayout, ImageReader, RgbaImage, codecs::jpeg::JpegEncoder};
use servo::RenderingContext;

use tracing::debug;

// Error
use amarotherum_error::AmaroError;
use miette::Result;

pub fn webview_to_image(state: &ServoState) -> Result<Handle, AmaroError> {
    let webviews = state.webviews.borrow();
    let (_id, webview) = webviews.last().unwrap();

    let _ = state.rendering_context.prepare_for_rendering();
    debug!("LoadStatus={:#?}", webview.load_status());
    webview.paint();
    let _ = state.rendering_context.present();

    // Servo image
    let rect = state.rect();
    let image = DynamicImage::ImageRgba8(state.rendering_context.read_to_image(rect).unwrap());
    let w = image.width();
    let h = image.height();
    let bytes = image.to_rgba8().into_raw();

    // let image = ImageReader::open("/home/anon/Pictures/hd_diner_de_con.jpg")?
    //     .decode()
    //     .unwrap();
    // let bytes = image.into_bytes();

    // let bytes = fs::read("/home/anon/Pictures/hd_diner_de_con.jpg").unwrap();

    // println!("{:?}", bytes);
    // let handler = Handle::from_bytes(bytes);
    let handler = Handle::from_rgba(w, h, bytes);

    Ok(handler)
}
