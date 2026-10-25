use tracing::info;

// Clap
use crate::gui;
use crate::utils::trace;

use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
use clap_verbosity_flag::{InfoLevel, Verbosity};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Error
use amarotherum_error::AmaroError;
use miette::{IntoDiagnostic, Result};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: Commands,
    #[command(flatten)]
    pub verbose: Verbosity,
}

#[derive(Debug, Subcommand, Clone, Eq, PartialEq)]
pub enum Commands {
    /// Operations on nodes
    #[command(subcommand)]
    Gui(GuiCommands),
}

#[derive(Debug, Subcommand, Clone, Eq, PartialEq)]
pub enum GuiCommands {
    Run,
}

impl Cli {
    pub fn run() -> Result<(), AmaroError> {
        trace::tracer()
            .verbosity(tracing::Level::TRACE)
            .set()
            .unwrap();
        info!("Starting cli.");
        let cli = Cli::parse();
        Self::switch(cli)?;
        Ok(())
    }
    pub fn switch(cli: Cli) -> Result<(), AmaroError> {
        match cli.commands {
            Commands::Gui(gui_command) => match gui_command {
                GuiCommands::Run => {
                    gui::main()?;
                }
            },
        }
        Ok(())
    }
}
