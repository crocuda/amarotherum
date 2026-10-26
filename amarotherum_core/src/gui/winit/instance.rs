use iced_core as core;
use iced_debug as debug;
use iced_futures as futures;
use iced_graphics as graphics;
use iced_program as program;
use iced_runtime as runtime;
use iced_winit::winit;

use iced_winit::clipboard;
use iced_winit::conversion;

use iced_winit::Error;
use iced_winit::Proxy;
use iced_winit::clipboard::Clipboard;

use iced::Program;
use iced_winit::core::backend;
use iced_winit::core::mouse;
use iced_winit::core::renderer;
use iced_winit::core::shell;
use iced_winit::core::theme;
use iced_winit::core::time::{Duration, Instant};
use iced_winit::core::widget::operation::{self, Operation};
use iced_winit::core::{Point, Renderer, Size};
use iced_winit::futures::futures::channel::mpsc;
use iced_winit::futures::futures::channel::oneshot;
use iced_winit::futures::futures::task;
use iced_winit::futures::futures::{Future, StreamExt};
use iced_winit::futures::subscription;
use iced_winit::futures::{Executor, Runtime};
use iced_winit::graphics::{Compositor, Shell, compositor};
use iced_winit::runtime::font;
use iced_winit::runtime::image;
use iced_winit::runtime::system;
use iced_winit::runtime::user_interface::{self, UserInterface};
use iced_winit::runtime::{Action, Task};

use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::slice;
use std::sync::Arc;

async fn run_instance<P>(
    mut program: program::Instance<P>,
    mut runtime: Runtime<P::Executor, Proxy<P::Message>, Action<P::Message>>,
    mut proxy: Proxy<P::Message>,
    mut event_receiver: mpsc::UnboundedReceiver<Event<Action<P::Message>>>,
    mut control_sender: mpsc::UnboundedSender<Control>,
    display_handle: winit::event_loop::OwnedDisplayHandle,
    is_daemon: bool,
    backend_settings: backend::Settings,
    mut renderer_settings: renderer::Settings,
    default_fonts: Vec<Cow<'static, [u8]>>,
    mut _system_theme: oneshot::Receiver<theme::Mode>,
) where
    P: Program + 'static,
    P::Theme: theme::Base,
{
    use winit::event;
    use winit::event_loop::ControlFlow;

    let mut window_manager = window::Manager::new();
    let mut is_window_opening = !is_daemon;

    let mut compositor = None;
    let mut events = Vec::new();
    let mut messages = shell::Bus::new();
    let mut actions = 0;

    let mut ui_caches = FxHashMap::default();
    let mut user_interfaces = ManuallyDrop::new(FxHashMap::default());
    let mut clipboard = Clipboard::new();

    #[cfg(all(feature = "linux-theme-detection", target_os = "linux"))]
    let mut system_theme = {
        let to_mode = |color_scheme| match color_scheme {
            mundy::ColorScheme::NoPreference => theme::Mode::None,
            mundy::ColorScheme::Light => theme::Mode::Light,
            mundy::ColorScheme::Dark => theme::Mode::Dark,
        };

        runtime.run(
            mundy::Preferences::stream(mundy::Interest::ColorScheme)
                .map(move |preferences| {
                    Action::System(system::Action::NotifyTheme(to_mode(
                        preferences.color_scheme,
                    )))
                })
                .boxed(),
        );

        runtime
            .enter(|| {
                mundy::Preferences::once_blocking(
                    mundy::Interest::ColorScheme,
                    core::time::Duration::from_millis(200),
                )
            })
            .map(|preferences| to_mode(preferences.color_scheme))
            .unwrap_or_default()
    };

    #[cfg(not(all(feature = "linux-theme-detection", target_os = "linux")))]
    let mut system_theme = _system_theme.try_recv().ok().flatten().unwrap_or_default();

    log::info!("System theme: {system_theme:?}");

    'next_event: loop {
        // Empty the queue if possible
        let event = if let Ok(event) = event_receiver.try_recv() {
            Some(event)
        } else {
            event_receiver.next().await
        };

        let Some(event) = event else {
            break;
        };

        match event {
            Event::WindowCreated {
                id,
                window,
                exit_on_close_request,
                make_visible,
                on_open,
            } => {
                if compositor.is_none() {
                    let (compositor_sender, compositor_receiver) = oneshot::channel();

                    let create_compositor = {
                        let window = window.clone();
                        let backend_settings = backend_settings.clone();
                        let display_handle = display_handle.clone();
                        let proxy = proxy.clone();
                        let default_fonts = default_fonts.clone();

                        async move {
                            let shell = Shell::new(proxy.clone());

                            let mut compositor =
                                <P::Renderer as compositor::Default>::Compositor::new(
                                    backend_settings,
                                    display_handle,
                                    window,
                                    shell,
                                )
                                .await;

                            if let Ok(compositor) = &mut compositor {
                                for font in default_fonts {
                                    compositor.load_font(font.clone());
                                }
                            }

                            compositor_sender
                                .send(compositor)
                                .ok()
                                .expect("Send compositor");

                            // HACK! Send a proxy event on completion to trigger
                            // a runtime re-poll
                            // TODO: Send compositor through proxy (?)
                            {
                                let (sender, _receiver) = oneshot::channel();

                                proxy.send_action(Action::Window(
                                    runtime::window::Action::GetLatest(sender),
                                ));
                            }
                        }
                    };

                    #[cfg(target_arch = "wasm32")]
                    wasm_bindgen_futures::spawn_local(create_compositor);

                    #[cfg(not(target_arch = "wasm32"))]
                    runtime.block_on(create_compositor);

                    match compositor_receiver.await.expect("Wait for compositor") {
                        Ok(new_compositor) => {
                            compositor = Some(new_compositor);
                        }
                        Err(error) => {
                            let _ = control_sender.start_send(Control::Crash(error.into()));
                            continue;
                        }
                    }
                }

                let window_theme = window
                    .theme()
                    .map(conversion::theme_mode)
                    .unwrap_or_default();

                if system_theme != window_theme {
                    system_theme = window_theme;

                    runtime.broadcast(subscription::Event::SystemThemeChanged(window_theme));
                }

                let is_first = window_manager.is_empty();
                let window = window_manager.insert(
                    id,
                    window,
                    &program,
                    compositor.as_mut().expect("Compositor must be initialized"),
                    proxy.clone(),
                    renderer_settings,
                    exit_on_close_request,
                    system_theme,
                );

                window
                    .raw
                    .set_theme(conversion::window_theme(window.state.theme_mode()));

                debug::theme_changed(|| {
                    if is_first {
                        theme::Base::seed(window.state.theme())
                    } else {
                        None
                    }
                });

                let logical_size = window.state.logical_size();

                window.renderer.hint(window.state.scale());

                let _ = user_interfaces.insert(
                    id,
                    build_user_interface(
                        &program,
                        user_interface::Cache::default(),
                        &mut window.renderer,
                        logical_size,
                        id,
                    ),
                );
                let _ = ui_caches.insert(id, user_interface::Cache::default());

                if make_visible {
                    window.raw.set_visible(true);
                }

                events.push((
                    id,
                    core::Event::Window(window::Event::Opened {
                        position: window.position(),
                        size: window.state.logical_size(),
                        scale_factor: window.raw.scale_factor() as f32,
                    }),
                ));

                let _ = on_open.send(id);
                is_window_opening = false;
            }
            Event::EventLoopAwakened(event) => {
                match event {
                    event::Event::NewEvents(event::StartCause::Init) => {
                        for (_id, window) in window_manager.iter_mut() {
                            window.raw.request_redraw();
                        }
                    }
                    event::Event::NewEvents(event::StartCause::ResumeTimeReached { .. }) => {
                        let now = Instant::now();

                        for (_id, window) in window_manager.iter_mut() {
                            if let Some(redraw_at) = window.redraw_at
                                && redraw_at <= now
                            {
                                window.raw.request_redraw();
                                window.redraw_at = None;
                            }
                        }

                        if let Some(redraw_at) = window_manager.redraw_at() {
                            let _ = control_sender
                                .start_send(Control::ChangeFlow(ControlFlow::WaitUntil(redraw_at)));
                        } else {
                            let _ =
                                control_sender.start_send(Control::ChangeFlow(ControlFlow::Wait));
                        }
                    }
                    event::Event::PlatformSpecific(event::PlatformSpecific::MacOS(
                        event::MacOS::ReceivedUrl(url),
                    )) => {
                        runtime.broadcast(subscription::Event::PlatformSpecific(
                            subscription::PlatformSpecific::MacOS(
                                subscription::MacOS::ReceivedUrl(url),
                            ),
                        ));
                    }
                    event::Event::UserEvent(action) => {
                        run_action(
                            action,
                            &program,
                            &proxy,
                            &mut runtime,
                            &mut compositor,
                            &mut events,
                            &mut messages,
                            &mut clipboard,
                            &mut control_sender,
                            &mut user_interfaces,
                            &mut window_manager,
                            &mut ui_caches,
                            &mut is_window_opening,
                            &mut system_theme,
                            &mut renderer_settings,
                        );
                        actions += 1;
                    }
                    event::Event::WindowEvent {
                        window_id: id,
                        event: event::WindowEvent::RedrawRequested,
                        ..
                    } => {
                        let Some(mut current_compositor) = compositor.as_mut() else {
                            continue;
                        };

                        let Some((id, mut window)) = window_manager.get_mut_alias(id) else {
                            continue;
                        };

                        let physical_size = window.state.physical_size();
                        let mut logical_size = window.state.logical_size();

                        if physical_size.width == 0 || physical_size.height == 0 {
                            continue;
                        }

                        // Window was resized between redraws
                        if window.surface_version != window.state.surface_version() {
                            window.renderer.hint(window.state.scale());

                            let ui = user_interfaces.remove(&id).expect("Remove user interface");

                            let layout_span = debug::layout(id);
                            let _ = user_interfaces
                                .insert(id, ui.relayout(logical_size, &mut window.renderer));
                            layout_span.finish();

                            current_compositor.configure_surface(
                                &mut window.surface,
                                physical_size.width,
                                physical_size.height,
                            );

                            window.surface_version = window.state.surface_version();
                        }

                        let redraw_event =
                            core::Event::Window(window::Event::RedrawRequested(Instant::now()));

                        let cursor = window.state.cursor();

                        let mut interface =
                            user_interfaces.get_mut(&id).expect("Get user interface");

                        let interact_span = debug::interact(id);
                        let mut redraw_count = 0;

                        let state = loop {
                            let message_count = messages.len();
                            let (state, _) = interface.update(
                                &window.raw,
                                &window.waker,
                                slice::from_ref(&redraw_event),
                                cursor,
                                &mut window.renderer,
                                &mut messages,
                            );

                            if redraw_count >= 2 {
                                log::warn!(
                                    "3 consecutive RedrawRequested events produced invalidation"
                                );

                                break state;
                            }

                            if message_count == messages.len() {
                                match state {
                                    user_interface::State::Outdated => {}
                                    user_interface::State::Updated { change, .. } => match change {
                                        user_interface::Change::None => break state,
                                        user_interface::Change::Overlay => {
                                            redraw_count += 1;
                                            continue;
                                        }
                                        user_interface::Change::Layout => {}
                                    },
                                }
                            }

                            redraw_count += 1;

                            if !messages.is_empty()
                                || matches!(state, user_interface::State::Outdated)
                            {
                                let caches: FxHashMap<_, _> =
                                    ManuallyDrop::into_inner(user_interfaces)
                                        .into_iter()
                                        .map(|(id, interface)| (id, interface.into_cache()))
                                        .collect();

                                let actions = update(&mut program, &mut runtime, &mut messages);

                                user_interfaces = ManuallyDrop::new(build_user_interfaces(
                                    &program,
                                    &mut window_manager,
                                    caches,
                                    &mut proxy,
                                ));

                                for action in actions {
                                    // Defer all window actions to avoid compositor
                                    // race conditions while redrawing
                                    if let Action::Window(_) = action {
                                        proxy.send_action(action);
                                        continue;
                                    }

                                    run_action(
                                        action,
                                        &program,
                                        &proxy,
                                        &mut runtime,
                                        &mut compositor,
                                        &mut events,
                                        &mut messages,
                                        &mut clipboard,
                                        &mut control_sender,
                                        &mut user_interfaces,
                                        &mut window_manager,
                                        &mut ui_caches,
                                        &mut is_window_opening,
                                        &mut system_theme,
                                        &mut renderer_settings,
                                    );
                                }

                                for (window_id, window) in window_manager.iter_mut() {
                                    // We are already redrawing this window
                                    if window_id == id {
                                        continue;
                                    }

                                    window.raw.request_redraw();
                                }

                                let Some(next_compositor) = compositor.as_mut() else {
                                    continue 'next_event;
                                };

                                current_compositor = next_compositor;
                                window = window_manager.get_mut(id).unwrap();

                                // Window scale factor changed during a redraw request
                                if logical_size != window.state.logical_size() {
                                    logical_size = window.state.logical_size();

                                    log::debug!(
                                        "Window scale factor changed during a redraw request"
                                    );

                                    let ui =
                                        user_interfaces.remove(&id).expect("Remove user interface");

                                    let layout_span = debug::layout(id);
                                    let _ = user_interfaces.insert(
                                        id,
                                        ui.relayout(logical_size, &mut window.renderer),
                                    );
                                    layout_span.finish();
                                }

                                interface = user_interfaces.get_mut(&id).unwrap();
                            }
                        };
                        interact_span.finish();

                        let draw_span = debug::draw(id);
                        interface.draw(
                            &mut window.renderer,
                            window.state.theme(),
                            &renderer::Style {
                                text_color: window.state.text_color(),
                            },
                            cursor,
                        );
                        draw_span.finish();

                        if let user_interface::State::Updated {
                            redraw_request,
                            input_method,
                            mouse_interaction,
                            clipboard: clipboard_requests,
                            ..
                        } = state
                        {
                            window.request_redraw(redraw_request);
                            window.request_input_method(input_method);
                            window.update_mouse(mouse_interaction);

                            run_clipboard(&mut proxy, &mut clipboard, clipboard_requests, id);
                        }

                        runtime.broadcast(subscription::Event::Interaction {
                            window: id,
                            event: redraw_event,
                            status: core::event::Status::Ignored,
                        });

                        window.draw_preedit();

                        let present_span = debug::present(id);
                        match current_compositor.present(
                            &mut window.renderer,
                            &mut window.surface,
                            window.state.viewport(),
                            window.state.background_color(),
                            || window.raw.pre_present_notify(),
                        ) {
                            Ok(()) => {
                                present_span.finish();
                            }
                            Err(error) => match error {
                                compositor::SurfaceError::OutOfMemory => {
                                    // This is an unrecoverable error.
                                    panic!("{error:?}");
                                }
                                compositor::SurfaceError::Occluded => {
                                    present_span.finish();

                                    // Do nothing and wait for window to become visible again
                                }
                                compositor::SurfaceError::Timeout => {
                                    present_span.finish();

                                    window.raw.request_redraw();
                                }
                                compositor::SurfaceError::Lost
                                | compositor::SurfaceError::Outdated
                                | compositor::SurfaceError::Other => {
                                    present_span.finish();

                                    // Recreate at most once a second, and do not request a redraw in
                                    // between, so a surface that keeps failing cannot spin the loop.
                                    let due = matches!(error, compositor::SurfaceError::Outdated)
                                        || window
                                            .surface_error_at
                                            .is_none_or(|at| at.elapsed() > Duration::from_secs(1));

                                    if due {
                                        window.surface_error_at = Some(Instant::now());

                                        log::warn!(
                                            "Error {error:?} when presenting surface. Recovering it."
                                        );

                                        let physical_size = window.state.physical_size();

                                        if matches!(error, compositor::SurfaceError::Outdated) {
                                            current_compositor.configure_surface(
                                                &mut window.surface,
                                                physical_size.width,
                                                physical_size.height,
                                            );
                                        } else {
                                            window.surface = current_compositor.create_surface(
                                                window.raw.clone(),
                                                physical_size.width,
                                                physical_size.height,
                                            );
                                        }

                                        window.raw.request_redraw();
                                    }
                                }
                            },
                        }
                    }
                    event::Event::WindowEvent {
                        event: window_event,
                        window_id,
                    } => {
                        if !is_daemon
                            && matches!(window_event, winit::event::WindowEvent::Destroyed)
                            && !is_window_opening
                            && window_manager.is_empty()
                        {
                            control_sender
                                .start_send(Control::Exit)
                                .expect("Send control action");

                            continue;
                        }

                        let Some((id, window)) = window_manager.get_mut_alias(window_id) else {
                            continue;
                        };

                        match window_event {
                            winit::event::WindowEvent::Resized(_)
                            | winit::event::WindowEvent::Occluded(false) => {
                                window.raw.request_redraw();
                            }
                            winit::event::WindowEvent::ThemeChanged(theme) => {
                                let mode = conversion::theme_mode(theme);

                                if mode != system_theme {
                                    system_theme = mode;

                                    runtime
                                        .broadcast(subscription::Event::SystemThemeChanged(mode));
                                }
                            }
                            _ => {}
                        }

                        if matches!(window_event, winit::event::WindowEvent::CloseRequested)
                            && window.exit_on_close_request
                        {
                            run_action(
                                Action::Window(runtime::window::Action::Close(id)),
                                &program,
                                &proxy,
                                &mut runtime,
                                &mut compositor,
                                &mut events,
                                &mut messages,
                                &mut clipboard,
                                &mut control_sender,
                                &mut user_interfaces,
                                &mut window_manager,
                                &mut ui_caches,
                                &mut is_window_opening,
                                &mut system_theme,
                                &mut renderer_settings,
                            );
                        } else {
                            window.state.update(&program, &window.raw, &window_event);

                            if let Some(event) = conversion::window_event(
                                window_event,
                                window.state.scale_factor(),
                                window.state.modifiers(),
                            ) {
                                events.push((id, event));
                            }
                        }
                    }
                    event::Event::AboutToWait => {
                        if actions > 0 {
                            proxy.free_slots(actions);
                            actions = 0;
                        }

                        if events.is_empty() && messages.is_empty() && window_manager.is_idle() {
                            continue;
                        }

                        let mut uis_stale = false;

                        for (id, window) in window_manager.iter_mut() {
                            let interact_span = debug::interact(id);
                            let mut window_events = vec![];

                            events.retain(|(window_id, event)| {
                                if *window_id == id {
                                    window_events.push(event.clone());
                                    false
                                } else {
                                    true
                                }
                            });

                            if window_events.is_empty() {
                                continue;
                            }

                            let (ui_state, statuses) = user_interfaces
                                .get_mut(&id)
                                .expect("Get user interface")
                                .update(
                                    &window.raw,
                                    &window.waker,
                                    &window_events,
                                    window.state.cursor(),
                                    &mut window.renderer,
                                    &mut messages,
                                );

                            #[cfg(feature = "unconditional-rendering")]
                            window.request_redraw(window::RedrawRequest::NextFrame);

                            match ui_state {
                                user_interface::State::Updated {
                                    redraw_request: _redraw_request,
                                    mouse_interaction,
                                    clipboard: clipboard_requests,
                                    ..
                                } => {
                                    window.update_mouse(mouse_interaction);

                                    #[cfg(not(feature = "unconditional-rendering"))]
                                    window.request_redraw(_redraw_request);

                                    run_clipboard(
                                        &mut proxy,
                                        &mut clipboard,
                                        clipboard_requests,
                                        id,
                                    );
                                }
                                user_interface::State::Outdated => {
                                    uis_stale = true;
                                }
                            }

                            for (event, status) in window_events.into_iter().zip(statuses) {
                                runtime.broadcast(subscription::Event::Interaction {
                                    window: id,
                                    event,
                                    status,
                                });
                            }

                            interact_span.finish();
                        }

                        for (id, event) in events.drain(..) {
                            runtime.broadcast(subscription::Event::Interaction {
                                window: id,
                                event,
                                status: core::event::Status::Ignored,
                            });
                        }

                        if !messages.is_empty() || uis_stale {
                            let cached_interfaces: FxHashMap<_, _> =
                                ManuallyDrop::into_inner(user_interfaces)
                                    .into_iter()
                                    .map(|(id, ui)| (id, ui.into_cache()))
                                    .collect();

                            let actions = update(&mut program, &mut runtime, &mut messages);

                            user_interfaces = ManuallyDrop::new(build_user_interfaces(
                                &program,
                                &mut window_manager,
                                cached_interfaces,
                                &mut proxy,
                            ));

                            for action in actions {
                                run_action(
                                    action,
                                    &program,
                                    &proxy,
                                    &mut runtime,
                                    &mut compositor,
                                    &mut events,
                                    &mut messages,
                                    &mut clipboard,
                                    &mut control_sender,
                                    &mut user_interfaces,
                                    &mut window_manager,
                                    &mut ui_caches,
                                    &mut is_window_opening,
                                    &mut system_theme,
                                    &mut renderer_settings,
                                );
                            }

                            for (_id, window) in window_manager.iter_mut() {
                                window.raw.request_redraw();
                            }
                        }

                        if let Some(redraw_at) = window_manager.redraw_at() {
                            let _ = control_sender
                                .start_send(Control::ChangeFlow(ControlFlow::WaitUntil(redraw_at)));
                        } else {
                            let _ =
                                control_sender.start_send(Control::ChangeFlow(ControlFlow::Wait));
                        }
                    }
                    _ => {}
                }
            }
            Event::Exit => break,
        }
    }

    let _ = ManuallyDrop::into_inner(user_interfaces);
}

/// Builds a window's [`UserInterface`] for the [`Program`].
fn build_user_interface<'a, P: Program>(
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

fn update<P: Program, E: Executor>(
    program: &mut program::Instance<P>,
    runtime: &mut Runtime<E, Proxy<P::Message>, Action<P::Message>>,
    messages: &mut shell::Bus<P::Message>,
) -> Vec<Action<P::Message>>
where
    P::Theme: theme::Base,
{
    use futures::futures;

    let mut actions = Vec::new();
    let mut outputs = Vec::new();

    while !messages.is_empty() {
        for (message, _receipt) in messages.drain() {
            let task = runtime.enter(|| program.update(message));

            if let Some(mut stream) = runtime::task::into_stream(task) {
                let waker = futures::task::noop_waker_ref();
                let mut context = futures::task::Context::from_waker(waker);

                // Run immediately available actions synchronously (e.g. widget operations)
                loop {
                    match runtime.enter(|| stream.poll_next_unpin(&mut context)) {
                        futures::task::Poll::Ready(Some(Action::Output(output))) => {
                            outputs.push(output);
                        }
                        futures::task::Poll::Ready(Some(action)) => {
                            actions.push(action);
                        }
                        futures::task::Poll::Ready(None) => {
                            break;
                        }
                        futures::task::Poll::Pending => {
                            runtime.run(stream);
                            break;
                        }
                    }
                }
            }
        }

        for output in outputs.drain(..) {
            let _ = messages.push(output);
        }
    }

    let subscription = runtime.enter(|| program.subscription());
    let recipes = subscription::into_recipes(subscription.map(Action::Output));

    runtime.track(recipes);

    actions
}
