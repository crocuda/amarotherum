mod components;
mod program;
pub mod state;
pub mod winit;

/// Launch the event loop.
pub fn main() -> anyhow::Result<()> {
    components::App::run();
    Ok(())
}
