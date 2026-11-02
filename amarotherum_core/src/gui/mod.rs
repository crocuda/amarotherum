mod components;
mod program;
pub mod state;
pub mod winit;

/// Launch the event loop.
pub fn main() -> anyhow::Result<()> {
    // winit::run(iced_program::with_executor::<
    //     components::App,
    //     iced_futures::backend::native::tokio::Executor,
    // >(components::App::new()))?;

    let app = components::App::new();
    app.run()?;

    Ok(())
}
