use std::sync::{mpsc::RecvError, Arc, PoisonError};

use crate::state::State;
use log::error;
use pwm_db::db_base::error::DatabaseError;

#[derive(Debug)]
pub enum GuiError {
    LockFail(String),
    RecvFail(String),
    DatabaseError(String),
    NoFile,
    NoVault,
    Utf8Fail(String),
}

impl GuiError {
    pub fn display_error_or_print(state: Arc<State>, error: String) {
        if let Err(display_error) = State::add_error(state, error.to_string()) {
            error!(
                "Failed to display error \"{}\", because of error: \"{}\"",
                error.to_string(),
                display_error.to_string()
            );
        }
    }
}

impl std::fmt::Display for GuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::LockFail(msg) => f.write_fmt(std::format_args!("Failed to lock: {}", msg)),
            Self::RecvFail(msg) => f.write_fmt(std::format_args!("Failed to recv: {}", msg)),
            Self::DatabaseError(msg) => f.write_fmt(std::format_args!("Vault error: {}", msg)),
            Self::NoFile => f.write_str("No file selected"),
            Self::NoVault => f.write_str("No vault opened"),
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

impl From<RecvError> for GuiError {
    fn from(value: RecvError) -> Self {
        Self::RecvFail(value.to_string())
    }
}
