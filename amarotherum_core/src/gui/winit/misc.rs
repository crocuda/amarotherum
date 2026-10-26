use super::window;

use iced_core as core;
use iced_winit::winit;

use iced_winit::Proxy;
use iced_winit::clipboard::Clipboard;

use iced_winit::runtime::Action;

use std::sync::Arc;

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
pub fn system_information(graphics: compositor::Information) -> system::Information {
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

pub fn run_clipboard<Message: Send>(
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
