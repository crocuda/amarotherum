mod components;
mod runner;
pub mod state;
mod winit;

// mod runner;
// pub use runner::main;

/// Launch the event loop.
pub fn main() -> anyhow::Result<()> {
    winit::run(components::App::new());
    Ok(())
}
