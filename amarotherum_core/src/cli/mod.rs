mod types;
use crate::{gui, trace};
pub use types::{Cli, Commands, GuiCommands};

use tracing::info;

// Clap
use crate::trace::tracer;
use clap::Parser;

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

impl Cli {
    pub fn run() -> Result<(), AmaroError> {
        tracer().verbosity(tracing::Level::TRACE).set().unwrap();
        info!("Starting cli.");
        let cli = Cli::parse();
        Self::switch(cli)?;
        Ok(())
    }
    pub fn switch(cli: Cli) -> Result<(), AmaroError> {
        match cli.commands {
            Commands::Gui(gui_command) => match gui_command {
                GuiCommands::Run => {
                    gui::view::main()?;
                }
            },
        }
        Ok(())
    }
}
