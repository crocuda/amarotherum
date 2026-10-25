use bon::bon;
use iced_winit::winit;
use miette::{Diagnostic, Report, SourceOffset, SourceSpan};

use thiserror::Error;
use tracing::error;

#[derive(Debug, Error, Diagnostic)]
pub enum AmaroError {
    ////////////////////////////////
    // Lib native errors
    #[error(transparent)]
    #[diagnostic(transparent)]
    WrapError(#[from] WrapError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    LibError(#[from] LibError),

    ////////////////////////////////
    // Type convertion
    #[error(transparent)]
    #[diagnostic(code(parse::error))]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    #[diagnostic(code(serde::error))]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    CastError(#[from] CastError),

    #[error(transparent)]
    #[diagnostic(code(amarotherum::io::error))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(amarotherum::io::error))]
    Utf8Error(#[from] std::string::FromUtf8Error),

    ////////////////////////////////
    // Env var error
    // Mainly use to get ssh_auth_agent socket.
    #[error(transparent)]
    #[diagnostic(code(env::error))]
    EnvError(#[from] std::env::VarError),

    // Env var error
    // Mainly use to get ssh_auth_agent socket.
    #[error(transparent)]
    #[diagnostic(code(future::error))]
    JoinError(#[from] tokio::task::JoinError),

    #[error(transparent)]
    #[diagnostic(code(winit::error))]
    WinitError(#[from] winit::error::EventLoopError),

    // #[error(transparent)]
    // #[diagnostic(code(bevy::error))]
    // BevyError(#[from] bevy::Error),
    #[error(transparent)]
    #[diagnostic(code(anyhow::error))]
    AnyhowError(#[from] anyhow::Error),
}

/**
A config error with help higher origin
Can be recursively chained.
*/
#[derive(Debug, Error, Diagnostic)]
#[error("{}", message)]
#[diagnostic(code(amarotherum::wrap::error))]
pub struct WrapError {
    pub message: String,
    #[diagnostic_source]
    pub origin: Report,
    #[help]
    pub help: String,
}

#[bon]
impl WrapError {
    #[builder]
    pub fn new(msg: &str, help: &str, origin: Report) -> Self {
        Self {
            message: msg.to_owned(),
            help: help.to_owned(),
            origin,
        }
    }
}
/**
A root cause error with no inner origin
*/
#[derive(Debug, Error, Diagnostic)]
#[error("{}", message)]
#[diagnostic(code(amarotherum::lib::error))]
pub struct LibError {
    pub message: String,
    #[help]
    pub help: String,
}
#[bon]
impl LibError {
    #[builder]
    pub fn new(msg: &str, help: &str) -> Self {
        Self {
            message: msg.to_owned(),
            help: help.to_owned(),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum CastError {
    // Deserialize
    #[error(transparent)]
    #[diagnostic(transparent)]
    TomlError(#[from] TomlError),

    // Serialize
    #[error(transparent)]
    #[diagnostic(code(serialize::toml))]
    TomlSerError(#[from] toml::ser::Error),
}

/**
A TOML report type with hint, colors and code span.
For better configuration file debugging
*/
#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(cast::toml))]
#[error("Serde: Could not convert Toml into Rust types")]
pub struct TomlError {
    #[source]
    pub origin: toml::de::Error,
    #[label("here")]
    pub at: SourceSpan,
    #[source_code]
    pub src: String,
}
impl TomlError {
    pub fn new(e: toml::de::Error, src: &str) -> Self {
        if let Some(span) = e.span() {
            let line = span.start;
            let column = span.end;
            TomlError {
                at: SourceSpan::new(
                    SourceOffset::from_location(
                        //source
                        src, line, column,
                    ),
                    1,
                ),
                src: src.to_owned(),
                origin: e,
            }
        } else {
            TomlError {
                at: SourceSpan::new(0.into(), 0),
                src: src.to_owned(),
                origin: e,
            }
        }
    }
}
