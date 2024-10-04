// SPDX-License-Identifier: MPL-2.0

use std::{fs::read_to_string, path::PathBuf};

const APP_NAME: &str = "timesheettool";

pub fn load_config(config_path: Option<PathBuf>) -> Config {
    let config_toml: PartialConfig = config_path
        .or_else(|| dirs::config_local_dir().map(|dir| dir.join(APP_NAME).join("config.toml")))
        .and_then(|path| {
            log::debug!("Reading configuration at path {:?}", &path);
            match read_to_string(&path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => Some(config),
                    Err(err) => {
                        log::warn!("Could not parse config at path {:?} {err}", path);
                        None
                    }
                },
                Err(err) => {
                    log::trace!(
                        "Could not read path {path:?} (assuming no config file set yet) {err}"
                    );
                    None
                }
            }
        })
        .unwrap_or_default();

    let database_path = config_toml
        .database_path
        .or_else(|| {
            dirs::data_local_dir().map(|dir| dir.join(APP_NAME).join("timesheettool.db"))
        })
        .expect("OS data directory could not be determined, use config file to set a database file location");
    log::trace!("Config: database_path is {:?}", &database_path);

    let time_round_minutes = config_toml.time_round_minutes.unwrap_or(15);
    Config {
        database_path,
        time_round_minutes,
    }
}

pub struct Config {
    pub database_path: PathBuf,
    pub time_round_minutes: u32,
}

#[derive(Default, serde::Deserialize)]
struct PartialConfig {
    database_path: Option<PathBuf>,
    time_round_minutes: Option<u32>,
}
