mod types;
use tracing::debug;
pub use types::{Cli, Commands, GuiCommands};

// Clap
use clap::Parser;

// Gui
use crate::gui::run;

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

impl Cli {
    pub fn run() -> Result<(), AmaroError> {
        let cli = Cli::parse();
        Self::switch(cli)?;
        Ok(())
    }
    pub fn switch(cli: Cli) -> Result<(), AmaroError> {
        match cli.commands {
            Commands::Gui(gui_command) => match gui_command {
                GuiCommands::Run => {
                    run()?;
                }
            },
        }
        Ok(())
    }
}
