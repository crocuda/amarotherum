// mod running_app_state;
// mod window;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Servo
use servo::{Servo, ServoBuilder, WebView, WebViewBuilder};
// use servoshell::desktop;

use crate::config::Config;

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

// Gui
use bevy::prelude::*;

const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 800;

#[derive(Default, Clone)]
pub struct Gui;

// pub async fn gui() -> Gui {
//     Gui { run }
// }

impl GuiMethods {
    pub async fn run() -> Result<()> {
        // aws_lc_rs::default_provider()
        //     .install_default()
        //     .expect("failed to install rustls crypto provider");

        let initial_url = demo_support::resolve_initial_url(env!("CARGO_MANIFEST_DIR"))
            .expect("failed to resolve initial URL");

        // Anchor surfman/ANGLE to a HighPerformance-DX12 GPU and run Servo on it.
        // Bevy (forced to DX12 + HighPerformance below) lands on the same GPU, so the
        // shared handle opened on Bevy's RenderDevice stays single-GPU.
        let servo_state = build_servo(&initial_url);

        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "amarotherum".into(),
                ..Default::default()
            }),
            ..Default::default()
        }));

        app.add_systems(Startup, setup);

        app.run();
        Ok(())
    }
}
