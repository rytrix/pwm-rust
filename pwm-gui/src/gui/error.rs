use std::sync::{
    mpsc::{RecvError, SendError},
    Arc, PoisonError,
};

use crate::state::State;
use log::{debug, error};
use pwm_db::db_base::error::DatabaseError;

#[derive(Debug)]
pub enum GuiError {
    LockFail(String),
    RecvFail(String),
    SendFail(String),
    IoError(String),
    DatabaseError(String),
    NoFile,
    NoVault,
    StringError(String),
    PasswordNotSame,
    Utf8Fail(String),
}

impl GuiError {
    pub fn display_error_or_print(state: Arc<State>, error: GuiError) {
        match error {
            GuiError::SendFail(error) => {
                debug!("{}", error);
            }
            GuiError::RecvFail(error) => {
                debug!("{}", error);
            }
            _ => {
                if let Err(display_error) = State::add_error(state, error.to_string()) {
                    error!(
                        "Failed to display error \"{}\", because of error: \"{}\"",
                        error,
                        display_error.to_string()
                    );
                }
            }
        }
    }
}

impl std::fmt::Display for GuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::LockFail(msg) => f.write_fmt(std::format_args!("Failed to lock: {}", msg)),
            Self::RecvFail(msg) => f.write_fmt(std::format_args!("Failed to recv: {}", msg)),
            Self::SendFail(msg) => f.write_fmt(std::format_args!("Failed to send: {}", msg)),
            Self::IoError(msg) => f.write_fmt(std::format_args!("IO error: {}", msg)),
            Self::DatabaseError(msg) => f.write_fmt(std::format_args!("Vault error: {}", msg)),
            Self::NoFile => f.write_str("No file selected"),
            Self::NoVault => f.write_str("No vault opened"),
            Self::StringError(msg) => f.write_fmt(std::format_args!("{}", msg)),
            Self::PasswordNotSame => f.write_str("Passwords do not match"),
            Self::Utf8Fail(msg) => f.write_fmt(std::format_args!("{}", msg)),
        };
    }
}

impl std::error::Error for GuiError {}

impl<T> From<PoisonError<T>> for GuiError {
    fn from(value: PoisonError<T>) -> Self {
        Self::LockFail(value.to_string())
    }
}

impl From<DatabaseError> for GuiError {
    fn from(value: DatabaseError) -> Self {
        Self::DatabaseError(value.to_string())
    }
}

impl From<String> for GuiError {
    fn from(value: String) -> Self {
        Self::StringError(value)
    }
}

impl From<std::io::Error> for GuiError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value.to_string())
    }
}

impl From<RecvError> for GuiError {
    fn from(value: RecvError) -> Self {
        Self::RecvFail(value.to_string())
    }
}

impl<T> From<SendError<T>> for GuiError {
    fn from(value: SendError<T>) -> Self {
        Self::SendFail(value.to_string())
    }
}
