use crate::state::State;
use crate::timer::Timer;

use std::{
    path::PathBuf,
    sync::{mpsc::RecvError, Arc, PoisonError},
};

use eframe::egui;

use pwm_db::db_base::DatabaseError;
use pwm_lib::crypt_file::{decrypt_file, encrypt_file};

#[derive(Debug)]
pub enum GuiError {
    LockFail(String),
    RecvFail(String),
    DatabaseError(String),
    NoFile,
    NoVault,
}

impl GuiError {
    fn display_error_or_print(state: Arc<State>, error: String) {
        if let Err(display_error) = State::add_error(state, (error.to_string(), Timer::default())) {
            eprintln!(
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

pub struct Gui {
    scale: f32,
    state: Arc<State>,
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            scale: 1.8,
            state: Arc::new(State::default()),
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(self.scale);

            let _text_height = egui::TextStyle::Body
                .resolve(ui.style())
                .size
                .max(ui.spacing().interact_size.y);

            // Top Bar
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Create").clicked() {
                        tokio::spawn(Self::file_new(self.state.clone()));
                    }
                    if ui.button("Open").clicked() {
                        tokio::spawn(Self::file_open(self.state.clone()));
                    }
                    if ui.button("Save").clicked() {
                        tokio::spawn(Self::file_save(self.state.clone()));
                    }
                    if ui.button("Save As").clicked() {
                        tokio::spawn(Self::file_save_as(self.state.clone()));
                    }
                });

                ui.menu_button("Options", |ui| {
                    ui.add(egui::Slider::new(&mut self.scale, 1.0..=3.0).text("UI Scale"));
                });

                ui.menu_button("Encryption", |ui| {
                    if ui.button("Encrypt File").clicked() {
                        tokio::spawn(Self::encrypt_file(self.state.clone()));
                    }
                    if ui.button("Decrypt File").clicked() {
                        tokio::spawn(Self::decrypt_file(self.state.clone()));
                    }
                });
            });
            // Top Bar End

            if let Err(error) = State::display_password_prompts(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = State::display_errors(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = State::display_vault(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }
        });
    }
}

impl Gui {
    pub fn open_file_dialog(state: Arc<State>) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new();

        match std::env::current_dir() {
            Ok(path) => {
                dialog = dialog.set_directory(path);
            }
            Err(error) => {
                GuiError::display_error_or_print(
                    state.clone(),
                    format!("Could not open current directory: {}", error.to_string()),
                );
            }
        };

        let file = dialog.pick_file();
        if let Some(file) = &file {
            *match state.prev_file.lock() {
                Ok(prev_file) => prev_file,
                Err(_) => return None,
            } = Some(file.display().to_string());

            eprintln!("Selected file {}", file.display().to_string());
        }
        file
    }

    fn save_file_dialog(state: Arc<State>) -> Option<PathBuf> {
        let dialog = rfd::FileDialog::new();

        let mut dialog = match state.vault.lock() {
            Ok(vault) => {
                if let Some(vault) = &*vault {
                    let name = vault.name_buffer.clone();
                    dialog.set_file_name(name)
                } else {
                    dialog
                }
            }
            Err(_) => return None,
        };

        match std::env::current_dir() {
            Ok(path) => {
                dialog = dialog.set_directory(path);
            }
            Err(error) => {
                // TODO logging
                eprintln!("Could not open current directory: {}", error.to_string());
            }
        };

        let file = dialog.save_file();
        if let Some(file) = &file {
            *match state.prev_file.lock() {
                Ok(prev_file) => prev_file,
                Err(_) => return None,
            } = Some(file.display().to_string());

            eprintln!("Saving file to {}", file.display().to_string());
        }
        file
    }

    async fn file_new(state: Arc<State>) {
        let error = State::create_vault(state.clone()).await;
        if let Err(error) = error {
            GuiError::display_error_or_print(state, error.to_string());
        }
    }

    async fn file_open(state: Arc<State>) {
        let error = State::open_vault_from_file(state.clone()).await;

        if let Err(error) = error {
            GuiError::display_error_or_print(state, error.to_string());
        }
    }

    async fn file_save(state: Arc<State>) {
        let path = match state.clone().prev_file.lock() {
            Ok(path) => {
                if let Some(path) = &*path {
                    path.clone()
                } else {
                    GuiError::display_error_or_print(state, "No previous file".to_string());
                    return;
                }
            }
            Err(error) => {
                GuiError::display_error_or_print(
                    state,
                    format!("File save error: {}", error.to_string()),
                );
                return;
            }
        };

        if let Err(error) = State::save_vault_to_file(state.clone(), path.as_str()).await {
            GuiError::display_error_or_print(state, error.to_string());
        }
    }

    async fn file_save_as(state: Arc<State>) {
        let path = match Self::save_file_dialog(state.clone()) {
            Some(path) => path,
            None => return,
        };

        match State::save_vault_to_file(state.clone(), path.display().to_string().as_str()).await {
            Ok(()) => (),
            Err(error) => {
                GuiError::display_error_or_print(state, error.to_string());
            }
        }
    }

    async fn crypt_prelude(state: Arc<State>) -> Option<(String, String)> {
        let file = Self::open_file_dialog(state.clone());
        if let Some(file_path) = file {
            let file = match file_path.file_name() {
                Some(file) => match file.to_str() {
                    Some(file) => file.to_string(),
                    None => file_path.display().to_string(),
                },
                None => file_path.display().to_string(),
            };

            let receiver = match State::add_password_prompt(
                state.clone(),
                format!("Enter password for {}", file),
            ) {
                Ok(receiver) => receiver,
                Err(_) => return None,
            };

            let password = receiver.recv().unwrap();

            return Some((String::from(file), password));
        }

        None
    }

    async fn encrypt_file(state: Arc<State>) {
        if let Some((file, password)) = Self::crypt_prelude(state.clone()).await {
            match encrypt_file(file, None, password.as_bytes()) {
                Ok(()) => (),
                Err(error) => {
                    GuiError::display_error_or_print(state, error.to_string());
                }
            };
        }
    }

    async fn decrypt_file(state: Arc<State>) {
        if let Some((file, password)) = Self::crypt_prelude(state.clone()).await {
            match decrypt_file(file, None, password.as_bytes()) {
                Ok(()) => (),
                Err(_error) => {
                    GuiError::display_error_or_print(
                        state,
                        String::from("Failed to decrypt file, invalid password"),
                    );
                }
            };
        }
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        let mut senders = self.state.password.lock().unwrap();
        senders.clear();
    }
}
