//! Copy-Paste-Rework of iced_winit crate 0.15.0-dev

// Fix missing relative deps
mod action;
mod error;
mod instance;
mod proxy;
mod window;
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

#[derive(Debug)]
pub enum Event<Message: 'static> {
    WindowCreated {
        id: window::Id,
        window: Arc<winit::window::Window>,
        exit_on_close_request: bool,
        make_visible: bool,
        on_open: oneshot::Sender<window::Id>,
    },
    EventLoopAwakened(winit::event::Event<Message>),
    Exit,
}

#[derive(Debug)]
pub enum Control {
    ChangeFlow(winit::event_loop::ControlFlow),
    Exit,
    Crash(Error),
    CreateWindow {
        id: window::Id,
        settings: window::Settings,
        title: String,
        monitor: Option<winit::monitor::MonitorHandle>,
        on_open: oneshot::Sender<window::Id>,
        scale_factor: f32,
    },
    SetAutomaticWindowTabbing(bool),
}

struct Runner<Message: 'static> {
    instance: std::pin::Pin<Box<dyn Future<Output = ()>>>,
    context: task::Context<'static>,
    id: Option<String>,
    sender: mpsc::UnboundedSender<Event<Action<Message>>>,
    receiver: mpsc::UnboundedReceiver<Control>,
    error: Option<Error>,
    system_theme: Option<oneshot::Sender<theme::Mode>>,

    #[cfg(target_arch = "wasm32")]
    canvas: Option<web_sys::HtmlCanvasElement>,
}
impl<Message> Runner<Message> {
    fn process_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: Event<Action<Message>>,
    ) {
        if event_loop.exiting() {
            return;
        }

        self.sender.start_send(event).expect("Send event");

        loop {
            let poll = self.instance.as_mut().poll(&mut self.context);

            match poll {
                task::Poll::Pending => match self.receiver.try_recv() {
                    Ok(control) => match control {
                        Control::ChangeFlow(flow) => {
                            use winit::event_loop::ControlFlow;

                            match (event_loop.control_flow(), flow) {
                                (ControlFlow::WaitUntil(current), ControlFlow::WaitUntil(new))
                                    if current > Instant::now() && current < new => {}
                                (ControlFlow::WaitUntil(current), ControlFlow::Wait)
                                    if current > Instant::now() => {}
                                _ => {
                                    event_loop.set_control_flow(flow);
                                }
                            }
                        }
                        Control::CreateWindow {
                            id,
                            settings,
                            title,
                            scale_factor,
                            monitor,
                            on_open,
                        } => {
                            let exit_on_close_request = settings.exit_on_close_request;

                            let visible = settings.visible;

                            #[cfg(target_arch = "wasm32")]
                            let target = settings.platform_specific.target.clone();

                            let window_attributes = conversion::window_attributes(
                                settings,
                                &title,
                                scale_factor,
                                monitor.or(event_loop.primary_monitor()),
                                self.id.clone(),
                            )
                            .with_visible(false);

                            #[cfg(target_arch = "wasm32")]
                            let window_attributes = {
                                use winit::platform::web::WindowAttributesExtWebSys;
                                window_attributes.with_canvas(self.canvas.take())
                            };

                            log::info!(
                                "Window attributes for id `{id:#?}`: {window_attributes:#?}"
                            );

                            // On macOS, the `position` in `WindowAttributes` represents the "inner"
                            // position of the window; while on other platforms it's the "outer" position.
                            // We fix the inconsistency on macOS by positioning the window after creation.
                            #[cfg(target_os = "macos")]
                            let mut window_attributes = window_attributes;

                            #[cfg(target_os = "macos")]
                            let position = window_attributes.position.take();

                            let window = event_loop
                                .create_window(window_attributes)
                                .expect("Create window");

                            #[cfg(target_os = "macos")]
                            if let Some(position) = position {
                                window.set_outer_position(position);
                            }

                            #[cfg(target_arch = "wasm32")]
                            {
                                use winit::platform::web::WindowExtWebSys;

                                let canvas = window.canvas().expect("Get window canvas");

                                let _ = canvas.set_attribute(
                                    "style",
                                    "display: block; width: 100%; height: 100%",
                                );

                                let window = web_sys::window().unwrap();
                                let document = window.document().unwrap();
                                let body = document.body().unwrap();

                                let target = target.and_then(|target| {
                                    body.query_selector(&format!("#{target}"))
                                        .ok()
                                        .unwrap_or(None)
                                });

                                match target {
                                    Some(node) => {
                                        let _ = node
                                            .replace_with_with_node_1(&canvas)
                                            .expect(&format!("Could not replace #{}", node.id()));
                                    }
                                    None => {
                                        let _ = body
                                            .append_child(&canvas)
                                            .expect("Append canvas to HTML body");
                                    }
                                };
                            }

                            self.process_event(
                                event_loop,
                                Event::WindowCreated {
                                    id,
                                    window: Arc::new(window),
                                    exit_on_close_request,
                                    make_visible: visible,
                                    on_open,
                                },
                            );
                        }
                        Control::Exit => {
                            self.process_event(event_loop, Event::Exit);
                            event_loop.exit();
                            break;
                        }
                        Control::Crash(error) => {
                            self.error = Some(error);
                            event_loop.exit();
                        }
                        Control::SetAutomaticWindowTabbing(_enabled) => {
                            #[cfg(target_os = "macos")]
                            {
                                use winit::platform::macos::ActiveEventLoopExtMacOS;
                                event_loop.set_allows_automatic_window_tabbing(_enabled);
                            }
                        }
                    },
                    _ => {
                        break;
                    }
                },
                task::Poll::Ready(_) => {
                    event_loop.exit();
                    break;
                }
            };
        }
    }
}
impl<Message> winit::application::ApplicationHandler<Action<Message>> for Runner<Message> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(sender) = self.system_theme.take() {
            let _ = sender.send(
                event_loop
                    .system_theme()
                    .map(conversion::theme_mode)
                    .unwrap_or_default(),
            );
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        self.process_event(
            event_loop,
            Event::EventLoopAwakened(winit::event::Event::NewEvents(cause)),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        #[cfg(target_os = "windows")]
        let is_move_or_resize = matches!(
            event,
            winit::event::WindowEvent::Resized(_) | winit::event::WindowEvent::Moved(_)
        );

        self.process_event(
            event_loop,
            Event::EventLoopAwakened(winit::event::Event::WindowEvent { window_id, event }),
        );

        // TODO: Remove when unnecessary
        // On Windows, we emulate an `AboutToWait` event after every `Resized` event
        // since the event loop does not resume during resize interaction.
        // More details: https://github.com/rust-windowing/winit/issues/3272
        #[cfg(target_os = "windows")]
        {
            if is_move_or_resize {
                self.process_event(
                    event_loop,
                    Event::EventLoopAwakened(winit::event::Event::AboutToWait),
                );
            }
        }
    }

    fn user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        action: Action<Message>,
    ) {
        self.process_event(
            event_loop,
            Event::EventLoopAwakened(winit::event::Event::UserEvent(action)),
        );
    }

    fn received_url(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, url: String) {
        self.process_event(
            event_loop,
            Event::EventLoopAwakened(winit::event::Event::PlatformSpecific(
                winit::event::PlatformSpecific::MacOS(winit::event::MacOS::ReceivedUrl(url)),
            )),
        );
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.process_event(
            event_loop,
            Event::EventLoopAwakened(winit::event::Event::AboutToWait),
        );
    }
}

/// Runs a [`Program`] with the provided settings.
pub fn run<P>(program: P) -> Result<(), Error>
where
    P: Program + 'static,
    P::Theme: theme::Base,
{
    use winit::event_loop::EventLoop;

    let boot_span = debug::boot();
    let settings = program.settings();
    let window_settings = program.window();

    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("Create event loop");

    let backend_settings = backend::Settings::from(&settings);
    let renderer_settings = renderer::Settings::from(&settings);
    let display_handle = event_loop.owned_display_handle();

    let (proxy, worker) = Proxy::new(event_loop.create_proxy());

    #[cfg(feature = "debug")]
    {
        let proxy = proxy.clone();

        debug::on_hotpatch(move || {
            proxy.send_action(Action::Reload);
        });
    }

    let mut runtime = {
        let executor = P::Executor::new().map_err(Error::ExecutorCreationFailed)?;
        executor.spawn(worker);

        Runtime::new(executor, proxy.clone())
    };

    let (program, task) = runtime.enter(|| program::Instance::new(program));
    let is_daemon = window_settings.is_none();

    let task = if let Some(window_settings) = window_settings {
        let mut task = Some(task);

        let (_id, open) = runtime::window::open(window_settings);

        open.then(move |_| task.take().unwrap_or_else(Task::none))
    } else {
        task
    };

    if let Some(stream) = runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(subscription::into_recipes(
        runtime.enter(|| program.subscription().map(Action::Output)),
    ));

    let (event_sender, event_receiver) = mpsc::unbounded();
    let (control_sender, control_receiver) = mpsc::unbounded();
    let (system_theme_sender, system_theme_receiver) = oneshot::channel();

    let instance: std::pin::Pin<Box<dyn Future<Output = ()>>> = Box::pin(run_instance::<P>(
        program,
        runtime,
        proxy.clone(),
        event_receiver,
        control_sender,
        display_handle,
        is_daemon,
        backend_settings,
        renderer_settings,
        settings.fonts,
        system_theme_receiver,
    ));

    let context = task::Context::from_waker(task::noop_waker_ref());

    let runner = Runner {
        instance,
        context,
        id: settings.id,
        sender: event_sender,
        receiver: control_receiver,
        error: None,
        system_theme: Some(system_theme_sender),

        #[cfg(target_arch = "wasm32")]
        canvas: None,
    };

    boot_span.finish();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut runner = runner;
        let _ = event_loop.run_app(&mut runner);

        runner.error.map(Err).unwrap_or(Ok(()))
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let _ = event_loop.spawn_app(runner);

        Ok(())
    }
}

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

/// Returns true if the provided event should cause a [`Program`] to
/// exit.
pub fn user_force_quit(
    event: &winit::event::WindowEvent,
    _modifiers: winit::keyboard::ModifiersState,
) -> bool {
    match event {
        #[cfg(target_os = "macos")]
        winit::event::WindowEvent::KeyboardInput {
            event:
                winit::event::KeyEvent {
                    logical_key: winit::keyboard::Key::Character(c),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
            ..
        } if c == "q" && _modifiers.super_key() => true,
        _ => false,
    }
}

#[cfg(feature = "sysinfo")]
fn system_information(graphics: compositor::Information) -> system::Information {
    use sysinfo::{Process, System};

    let mut system = System::new_all();
    system.refresh_all();

    let cpu_brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_default();

    let memory_used = sysinfo::get_current_pid()
        .and_then(|pid| system.process(pid).ok_or("Process not found"))
        .map(Process::memory)
        .ok();

    system::Information {
        system_name: System::name(),
        system_kernel: System::kernel_version(),
        system_version: System::long_os_version(),
        system_short_version: System::os_version(),
        cpu_brand,
        cpu_cores: system.physical_core_count(),
        memory_total: system.total_memory(),
        memory_used,
        graphics_adapter: graphics.adapter,
        graphics_backend: graphics.backend,
    }
}

fn run_clipboard<Message: Send>(
    proxy: &mut Proxy<Message>,
    clipboard: &mut Clipboard,
    requests: core::Clipboard,
    window: window::Id,
) {
    for kind in requests.reads {
        let proxy = proxy.clone();

        clipboard.read(kind, move |result| {
            proxy.send_action(Action::Event {
                window,
                event: core::Event::Clipboard(core::clipboard::Event::Read(result.map(Arc::new))),
            });
        });
    }

    if let Some(content) = requests.write {
        let proxy = proxy.clone();

        clipboard.write(content, move |result| {
            proxy.send_action(Action::Event {
                window,
                event: core::Event::Clipboard(core::clipboard::Event::Written(result)),
            });
        });
    }
}
