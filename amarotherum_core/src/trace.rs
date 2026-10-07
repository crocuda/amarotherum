use bon::builder;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use std::fmt::Debug;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

// Error Handling
use miette::Result;
use tracing::{debug, error, info, trace};

/// Always print trace/logs inside test.
#[builder(finish_fn = set)]
pub fn tracer(
    verbosity: tracing::Level,
    /// Wether http request/response logs should be enabled
    http: Option<bool>,
) -> Result<()> {
    // Get tracing level from command line or fallback to function args.
    let env_var: Option<String> = std::env::var("AMAROTHERUM_TRACING").ok();
    let level: String = match env_var {
        Some(v) => v,
        None => verbosity.to_string().to_lowercase().to_owned(),
    };
    // Set crate filter
    let mut filter: String = level.clone();
    filter += ",users=warn";
    match http {
        Some(true) => filter += &format!(",tower_http={level},mio={level}"),
        Some(false) | None => filter += ",tower_http=warn,mio=warn",
    };
    filter += ",calloop=warn,zbus=warn";

    let builder = FmtSubscriber::builder()
        .with_max_level(verbosity)
        .with_env_filter(EnvFilter::try_new(filter).unwrap());

    let builder = builder.pretty();
    // let builder = builder.compact();
    let subscriber = builder.finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    Ok(())
}
