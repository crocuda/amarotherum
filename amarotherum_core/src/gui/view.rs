// mod running_app_state;
// mod window;

use std::boxed::Box;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bevy::window::{PrimaryWindow, RawHandleWrapper, WindowWrapper};
use bevy::winit::{WinitPlugin, WinitSettings, WinitWindows};
use dpi::PhysicalSize;
use raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle, WindowHandle};

// Servo
use servo::{
    Servo, ServoBuilder, WebView, WebViewBuilder, WebViewDelegate, WindowRenderingContext,
};
use url::Url;
// use servoshell::desktop;

use crate::config::Config;

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

// Gui
use bevy::prelude::*;

use bevy::{
    color::palettes::basic::*,
    picking::hover::Hovered,
    // platform::sync::Arc,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Activate, Button, observe},
    winit,
};

const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 800;

#[derive(Clone)]
pub struct Gui {
    window: Window,
}
impl Default for Gui {
    fn default() -> Self {
        let window = Window {
            title: "amarotherum".into(),
            ..Default::default()
        };
        Self { window }
    }
}

pub fn run() -> Result<(), AmaroError> {
    // let gui = Gui::default();

    // aws_lc_rs::default_provider()
    //     .install_default()
    //     .expect("failed to install rustls crypto provider");

    // let initial_url = demo_support::resolve_initial_url(env!("CARGO_MANIFEST_DIR"))
    //     .expect("failed to resolve initial URL");

    // let view = servo_view(gui);

    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    // app.add_plugins(DefaultPlugins.set(WindowPlugin {
    //     primary_window: Some(gui.window),
    //     ..Default::default()
    // }));

    // Individual Plugin listing
    // app.add_plugins(WinitPlugin::default());
    app.insert_resource(WinitSettings::desktop_app());

    app.add_systems(Startup, setup);
    // app.add_systems(Startup, get_window);

    app.run();
    Ok(())
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // A full-screen container that centers the button.
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            button(&assets),
            // React to the button being clicked. `Button` fires an `Activate` event on a
            // completed click, so we attach an observer for it right here on the button entity.
            //
            // Try it: this is where you'd run your own logic (start a game, open a menu, etc.).
            observe(|_activate: On<Activate>| {
                info!("Button clicked!");
            }),
        )],
    ));
}

fn setup_bis(mut commands: Commands, assets: Res<AssetServer>) {
    // A full-screen container that centers the button.
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(statusline(&assets))],
    ));
    // StatusLine placing.
    // commands.spawn((
    //     Node {
    //         width: percent(100),
    //         height: px(35),
    //         border: UiRect::all(px(0)),
    //         // horizontally center child text
    //         justify_content: JustifyContent::End,
    //         // vertically center child text
    //         align_items: AlignItems::End,
    //         // border_radius: BorderRadius::MAX,
    //         ..default()
    //     },
    //     children![(statusline(&assets))],
    // ));
}

fn get_window(mut windows: Query<&mut RawHandleWrapper, With<PrimaryWindow>>) {
    // Query returns one window typically.
    // for mut window in windows.iter_mut() {
    for window in windows.iter() {
        // servo_view(window);

        println!("window={:#?}", window);
        // let _winit = WinitWindows::new();
        // let window = _winit.get_window(entity);

        // let window = _winit.windows.values().next().unwrap();

        let servo = ServoBuilder::default().build();
        servo.setup_logging();

        // let window = WindowWrapper::<winit::Window>::new(window);
        // let window = WindowWrapper::new(window);
        unsafe {
            let handle = window;
            // let handle = RawHandleWrapper::new(&window).unwrap();
            //     let handle = handle.get_handle();
            //
            const DEFAULT_WIDTH: u32 = 1280;
            const DEFAULT_HEIGHT: u32 = 800;
            let size = PhysicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT);
            //
            let render_ctx = WindowRenderingContext::new(
                DisplayHandle::borrow_raw(handle.get_display_handle()),
                WindowHandle::borrow_raw(handle.get_window_handle()),
                size,
            )
            .unwrap();

            let url = Url::parse("https://niri-wm.github.io/").unwrap();
            let webview = WebViewBuilder::new(&servo, Rc::new(render_ctx))
                .hidpi_scale_factor(euclid::Scale::new(1.0))
                .url(url)
                .build();
        }
    }
}

fn statusline(asset_server: &AssetServer) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            height: px(40),
            border: UiRect::all(px(5)),
            // horizontally center child text
            justify_content: JustifyContent::Start,
            // vertically center child text
            align_items: AlignItems::Center,
            // border_radius: BorderRadius::MAX,
            ..default()
        },
        children![(
            Text::new("StatusLine"),
            TextFont {
                font_size: FontSize::Px(33.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}

// struct AppState {
//     window: Window,
//     servo: Servo,
//     rendering_context: Rc<WindowRenderingContext>,
//     webviews: RefCell<Vec<WebView>>,
// }
// impl WebViewDelegate for AppState {
//     fn notify_new_frame_ready(&self, _: WebView) {
//         self.window.request_redraw();
//     }
// }

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button(asset_server: &AssetServer) -> impl Bundle {
    (
        // The headless button widget. It handles pointer/keyboard input and pressed-state
        // tracking for us; we only provide the look below.
        Button,
        // `Hovered` is used by the picking backend to track if the pointer is over the button.
        Hovered::default(),
        Node {
            width: px(150),
            height: px(65),
            border: UiRect::all(px(5)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(Color::BLACK),
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new("Button"),
            TextFont {
                font_size: FontSize::Px(33.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}

/// Restyle the button and update its label to reflect its current state.
///
/// The `Button` widget maintains a `Pressed` component while the button is held down, and the
/// picking backend keeps `Hovered` up to date. We simply read those each frame and pick a look.
fn update_button_appearance(
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<Button>,
    >,
    mut text_query: Query<&mut Text>,
) {
    for (hovered, pressed, mut color, mut border_color, children) in &mut buttons {
        let Ok(mut text) = text_query.get_mut(children[0]) else {
            continue;
        };

        match (hovered.get(), pressed) {
            // Pressed (and, since you can only press what you're hovering, also hovered).
            (_, true) => {
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.set_all(RED);
            }
            // Hovered but not pressed.
            (true, false) => {
                **text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.set_all(WHITE);
            }
            // Neither hovered nor pressed.
            (false, false) => {
                **text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.set_all(BLACK);
            }
        }
    }
}
