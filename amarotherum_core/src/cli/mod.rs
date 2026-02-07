mod types;
pub use types::{Cli, Commands, GuiCommands};
mod utils;

// Clap
use clap::Parser;

// Gui
use crate::Gui;

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

impl Cli {
    pub async fn run() -> Result<(), AmaroError> {
        let cli = Cli::parse();
        Self::switch(cli).await?;
        Ok(())
    }
    pub async fn switch(cli: Cli) -> Result<(), AmaroError> {
        utils::set_tracer(&cli)?;
        utils::set_logger(&cli)?;

        match cli.commands {
            Commands::Gui(gui_command) => match gui_command {
                GuiCommands::Run => Gui::run().await?,
            },
        }
        Ok(())
    }
}
