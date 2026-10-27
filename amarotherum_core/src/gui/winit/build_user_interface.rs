use super::window;

use iced_core as core;
use iced_debug as debug;
use iced_futures as futures;
use iced_runtime as runtime;

use iced_winit::Proxy;

// use iced::Program;
// use iced_program as program;
use crate::gui::program::{self, Program};

use iced_winit::core::shell;
use iced_winit::core::theme;
use iced_winit::core::{Renderer, Size};
use iced_winit::futures::futures::StreamExt;
use iced_winit::futures::subscription;
use iced_winit::futures::{Executor, Runtime};
use iced_winit::graphics::Compositor;
use iced_winit::runtime::Action;
use iced_winit::runtime::user_interface::{self, UserInterface};

use rustc_hash::FxHashMap;

/// Build the user interface for every window.
pub fn build_user_interfaces<'a, P: Program, C>(
    program: &'a program::Instance<P>,
    window_manager: &mut window::Manager<P, C>,
    mut cached_user_interfaces: FxHashMap<window::Id, user_interface::Cache>,
    proxy: &mut Proxy<P::Message>,
) -> FxHashMap<window::Id, UserInterface<'a, P::Message, P::Theme, P::Renderer>>
where
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: theme::Base,
{
    for (id, window) in window_manager.iter_mut() {
        let old_size = window.state.logical_size();

        window.state.synchronize(program, id, &window.raw);

        let new_size = window.state.logical_size();

        if old_size != new_size {
            proxy.send_action(Action::Event {
                window: id,
                event: core::Event::Window(window::Event::Resized(new_size)),
            });
        }

        window.renderer.hint(window.state.scale());
    }

    debug::theme_changed(|| {
        window_manager
            .first()
            .and_then(|window| theme::Base::seed(window.state.theme()))
    });

    cached_user_interfaces
        .drain()
        .filter_map(|(id, cache)| {
            let window = window_manager.get_mut(id)?;

            Some((
                id,
                build_user_interface(
                    program,
                    cache,
                    &mut window.renderer,
                    window.state.logical_size(),
                    id,
                ),
            ))
        })
        .collect()
}

/// Builds a window's [`UserInterface`] for the [`Program`].
pub fn build_user_interface<'a, P: Program>(
    program: &'a program::Instance<P>,
    cache: user_interface::Cache,
    renderer: &mut P::Renderer,
    size: Size,
    id: window::Id,
) -> UserInterface<'a, P::Message, P::Theme, P::Renderer>
where
    P::Theme: theme::Base,
{
    let view_span = debug::view(id);
    let view = program.view(id);
    view_span.finish();

    let layout_span = debug::layout(id);
    let user_interface = UserInterface::build(view, size, cache, renderer);
    layout_span.finish();

    user_interface
}
