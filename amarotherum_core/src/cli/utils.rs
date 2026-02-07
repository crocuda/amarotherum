use super::Cli;

// Error handling
use amarotherum_error::AmaroError;
use miette::Result;

// Logger
use env_logger::Builder;

// Tracing
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub fn tracing_per_crate(verbosity: tracing::Level) -> Result<String, AmaroError> {
    let res = match verbosity {
        tracing::Level::TRACE => "",
        tracing::Level::DEBUG => "",
        tracing::Level::INFO => "",
        tracing::Level::WARN => "",
        tracing::Level::ERROR => "",
        _ => "",
    };
    Ok(res.to_owned())
}
pub fn logging_per_crate(verbosity: log::LevelFilter) -> Result<String, AmaroError> {
    let res = match verbosity {
        log::LevelFilter::Trace => "",
        log::LevelFilter::Debug => "",
        log::LevelFilter::Info => "",
        log::LevelFilter::Warn => "",
        log::LevelFilter::Error => "",
        _ => "",
    };
    Ok(res.to_owned())
}
/// Build tracing
pub fn set_tracer(cli: &Cli) -> Result<(), AmaroError> {
    // Set verbosity
    let verbosity: tracing::Level = cli.verbose.tracing_level().unwrap();
    let filter = format!(
        "{},{}",
        verbosity.to_string().to_lowercase(),
        tracing_per_crate(verbosity)?
    );
    let builder = FmtSubscriber::builder()
        .with_max_level(verbosity)
        // .with_file(false)
        .with_env_filter(EnvFilter::try_new(filter).unwrap());

    #[cfg(debug_assertions)]
    let builder = builder.pretty();
    #[cfg(not(debug_assertions))]
    let builder = builder.compact();

    let subscriber = builder.finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    Ok(())
}
/// Build logger
pub fn set_logger(cli: &Cli) -> Result<(), AmaroError> {
    // Set verbosity
    let verbosity: log::LevelFilter = cli.verbose.log_level_filter();
    // Disable sql logs
    let filter = format!(
        "{},{}",
        verbosity.to_string().to_lowercase(),
        logging_per_crate(verbosity)?
    );
    unsafe {
        std::env::set_var("AMAROTHERUM_LOG", filter);
    }
    Builder::from_env("AMAROTHERUM_LOG").init();

    Ok(())
}
