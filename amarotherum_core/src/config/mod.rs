// Config
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// Error handling
use amarotherum_error::{AmaroError, CastError, LibError, TomlError, WrapError};
use miette::{Error, Result};
use tracing::{error, trace};

/// The main clic onfiguration struct.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    gui: GuiConfig,
}
impl Config {
    /*
     * Get config from crate directory
     */
    fn debug_path() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("./test/amarotherum.config.toml");
        return path;
    }
    /*
     * Get config from FHS path.
     */
    fn release_path() -> PathBuf {
        let mut path = PathBuf::new();
        path.push("/etc/amarotherum/config.toml");
        return path;
    }
    /*
     * Return configuration from default file path.
     */
    pub fn get() -> Result<Self, AmaroError> {
        // let config = CONFIG.lock().unwrap().clone();
        // Ok(config)

        #[cfg(debug_assertions)]
        let path = Self::debug_path();

        #[cfg(not(debug_assertions))]
        let path = Self::release_path();

        let path = path.display().to_string();
        let config = match Self::from_file(&path) {
            Ok(v) => v,
            Err(e) => {
                let message = format!("Couldn't find a configuration file.",);
                let help = format!("Create a configuration file at /etc/amarotherum/config.toml");
                let err = WrapError::builder()
                    .msg(&message)
                    .help(&help)
                    .origin(Error::from_err(e))
                    .build();

                error!("{}", err);
                return Err(err.into());
            }
        };

        trace!("Found config file.");
        Ok(config)
    }
    pub fn from_file(path: &str) -> Result<Self, AmaroError> {
        let string = fs::read_to_string(path)?;
        Self::from_toml(&string)
    }
    pub fn from_toml(string: &str) -> Result<Self, AmaroError> {
        let res = toml::from_str::<Self>(&string);
        let item = match res {
            Ok(res) => res,
            Err(e) => {
                let err = CastError::TomlError(TomlError::new(e, &string));
                return Err(err.into());
            }
        };
        Ok(item)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuiConfig {
    theme: Theme,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    window: Window,
    tab: Tab,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tab {
    color: String,
    width: u8,
    border: Border,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Window {
    color: String,
    border: Border,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Border {
    color: String,
    width: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_from_file() -> Result<()> {
        let res = Config::get()?;
        println!("{:#?}", res);
        Ok(())
    }
}
