use std::env::VarError;

use json::JsonValue;
use log::{info, warn};

fn default_config() -> JsonValue {
    json::object! {
        dark: true,
        scale: 2.0
    }
}

fn get_config_dir() -> Result<std::path::PathBuf, VarError> {
    #[cfg(unix)]
    let app_data = std::env::var("HOME")? + "/.config";

    #[cfg(windows)]
    let app_data = std::env::var("APP_DATA")?;

    let mut path = std::path::PathBuf::from(app_data);
    path.push("pwm");

    if !path.exists() {
        match std::fs::create_dir_all(&path) {
            Ok(()) => (),
            Err(_error) => return Err(VarError::NotPresent),
        };
    }

    Ok(path)
}

fn get_config_file() -> Result<std::path::PathBuf, VarError> {
    let mut config_dir = get_config_dir()?;
    config_dir.push("config.json");

    Ok(config_dir)
}

pub fn get_config() -> JsonValue {
    let file = match get_config_file() {
        Ok(file) => file,
        Err(error) => {
            warn!("failed to get config: {}", error.to_string());
            return default_config();
        }
    };

    info!("reading from file \"{}\"", file.display().to_string());

    let file = match std::fs::read(file) {
        Ok(file) => file,
        Err(error) => {
            warn!("failed to get config: {}", error.to_string());
            return default_config();
        }
    };


    let file = match String::from_utf8(file) {
        Ok(file) => file,
        Err(error) => {
            warn!("failed to get config: {}", error.to_string());
            return default_config();
        }
    };

    let parsed = json::parse(file.as_str()).unwrap_or(default_config());

    parsed
}

pub fn write_config(config: JsonValue) {
    let file = match get_config_file() {
        Ok(file) => file,
        Err(error) => {
            warn!("failed to get config: {}", error.to_string());
            return;
        }
    };

    info!("writing to file \"{}\"", file.display().to_string());

    match std::fs::write(file, config.to_string()) {
        Ok(()) => (),
        Err(error) => {
            warn!("failed to get config: {}", error.to_string());
            return;
        }
    };
}
