use crate::state::State;
use crate::timer::Timer;

use std::{
    path::PathBuf,
    sync::{mpsc::RecvError, Arc, PoisonError},
};

use eframe::egui::{self, Layout, Vec2};
use log::{error, info, warn};

use pwm_db::db_base::DatabaseError;
use pwm_lib::{
    crypt_file::{decrypt_file, encrypt_file},
    random::random_password,
    zeroize::Zeroizing,
};

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
        if let Err(display_error) = State::add_error(state, (error.to_string(), Timer::default())) {
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

pub struct Gui {
    scale: f32,
    update_scale: bool,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    state: Arc<State>,
}

const GUI_SCALE: f32 = 2.0;

impl Default for Gui {
    fn default() -> Self {
        Self {
            scale: GUI_SCALE,
            update_scale: true,
            show_confirmation_dialog: false,
            allowed_to_close: false,
            state: Arc::new(State::default()),
        }
    }
}

fn was_vault_modified(state: Arc<State>) -> bool {
    let vault = match state.vault.lock() {
        Ok(vault) => vault,
        Err(error) => {
            error!("Failed to lock mutex: {}", error.to_string());
            return true;
        }
    };
    if let Some(vault) = &*vault {
        if vault.changed {
            return true;
        }
    }

    false
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ctx.input(|i| i.viewport().close_requested()) {
                if self.allowed_to_close {
                    // do nothing - we will close
                } else {
                    if was_vault_modified(self.state.clone()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.show_confirmation_dialog = true;
                    }
                }
            }

            if self.show_confirmation_dialog {
                egui::Window::new("")
                    .collapsible(false)
                    .auto_sized()
                    .resizable(false)
                    .title_bar(false)
                    .show(ctx, |ui| {
                        ui.allocate_ui_with_layout(
                            egui::Vec2 { x: 150.0, y: 50.0 },
                            Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.label("Vault has been modified");
                                ui.label("Exit anyways?");

                                ui.columns(2, |columns| {
                                    if columns[0]
                                        .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("Yes"))
                                        .clicked()
                                    {
                                        self.show_confirmation_dialog = false;
                                        self.allowed_to_close = true;
                                        columns[0]
                                            .ctx()
                                            .send_viewport_cmd(egui::ViewportCommand::Close);
                                    }

                                    if columns[1]
                                        .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("No"))
                                        .clicked()
                                    {
                                        self.show_confirmation_dialog = false;
                                        self.allowed_to_close = false;
                                    }
                                });
                            },
                        );
                    });
            }

            if self.update_scale {
                ctx.set_pixels_per_point(self.scale);
            }

            ui.output_mut(|o| {
                if let Ok(mut clipboard) = self.state.clipboard_string.lock() {
                    if let Some(result) = &mut *clipboard {
                        // println!("{}", result.as_str());
                        let string = result.to_string();
                        o.copied_text = string;
                        *clipboard = None;
                    }
                }
            });

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
                    if !ui
                        .add(egui::Slider::new(&mut self.scale, 1.0..=3.0).text("UI Scale"))
                        .dragged()
                    {
                        self.update_scale = true;
                    } else {
                        self.update_scale = false;
                    };
                });

                ui.menu_button("Encryption", |ui| {
                    if ui.button("Encrypt File").clicked() {
                        tokio::spawn(Self::encrypt_file(self.state.clone()));
                    }
                    if ui.button("Decrypt File").clicked() {
                        tokio::spawn(Self::decrypt_file(self.state.clone()));
                    }
                    match self.state.password_length.lock() {
                        Ok(mut password_length) => {
                            ui.menu_button("Password Generation", |ui| {
                                ui.label("Length");
                                ui.horizontal(|ui| {
                                    let response = ui.add_sized(
                                        [100.0, 20.0],
                                        egui::TextEdit::singleline(&mut *password_length),
                                    );
                                    if response.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        tokio::spawn(Self::random_password(self.state.clone()));
                                    }
                                    if ui.button("Generate").clicked() {
                                        tokio::spawn(Self::random_password(self.state.clone()));
                                    }
                                });
                            });
                        }
                        Err(error) => {
                            GuiError::display_error_or_print(self.state.clone(), error.to_string());
                        }
                    }
                });
            });

            ui.separator();
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
    pub fn open_file_dialog(state: Arc<State>, update_prev_file: bool) -> Option<PathBuf> {
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
        if update_prev_file {
            if let Some(file) = &file {
                *match state.prev_file.lock() {
                    Ok(prev_file) => prev_file,
                    Err(_) => return None,
                } = Some(file.display().to_string());

                info!("Selected file {}", file.display().to_string());
            }
        }
        file
    }

    pub fn save_file_dialog(state: Arc<State>, update_prev_file: bool) -> Option<PathBuf> {
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
                warn!("Could not open current directory: {}", error.to_string());
            }
        };

        let file = dialog.save_file();
        if update_prev_file {
            if let Some(file) = &file {
                *match state.prev_file.lock() {
                    Ok(prev_file) => prev_file,
                    Err(_) => return None,
                } = Some(file.display().to_string());

                info!("Selected save file \"{}\"", file.display().to_string());
            }
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
        let get_password = |state: Arc<State>| -> Result<Zeroizing<String>, GuiError> {
            let receiver = State::add_password_prompt(
                state.clone(),
                String::from("Enter master password to save vault"),
            )?;
            let password = receiver.recv()?;

            Ok(password)
        };

        let password = match get_password(state.clone()) {
            Ok(password) => password,
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
                return;
            }
        };

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

        if let Err(error) =
            State::save_vault_to_file(state.clone(), path.as_str(), password.as_bytes()).await
        {
            GuiError::display_error_or_print(state, error.to_string());
        }
    }

    async fn file_save_as(state: Arc<State>) {
        let get_password = |state: Arc<State>| -> Result<Zeroizing<String>, GuiError> {
            let receiver = State::add_password_prompt(
                state.clone(),
                String::from("Enter master password to save vault"),
            )?;
            let password = receiver.recv()?;

            Ok(password)
        };

        let password = match get_password(state.clone()) {
            Ok(password) => password,
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
                return;
            }
        };

        let path = match Self::save_file_dialog(state.clone(), true) {
            Some(path) => path,
            None => return,
        };

        match State::save_vault_to_file(
            state.clone(),
            path.display().to_string().as_str(),
            password.as_bytes(),
        )
        .await
        {
            Ok(()) => (),
            Err(error) => {
                GuiError::display_error_or_print(state, error.to_string());
            }
        }
    }

    async fn crypt_prelude(state: Arc<State>) -> Option<(String, Zeroizing<String>)> {
        let file = Self::open_file_dialog(state.clone(), true);
        if let Some(file_path) = file {
            let file = get_file_name(file_path);

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

    async fn random_password(state: Arc<State>) {
        let mut clipboard = match state.clipboard_string.lock() {
            Ok(clipboard) => clipboard,
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
                return;
            }
        };

        match state.password_length.lock() {
            Ok(password_length) => {
                let length: usize = match password_length.parse() {
                    Ok(length) => length,
                    Err(error) => {
                        GuiError::display_error_or_print(state.clone(), error.to_string());
                        return;
                    }
                };

                *clipboard = Some(Zeroizing::new(random_password(length)));
            }
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
            }
        }
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        let mut senders = self.state.password.lock().unwrap();
        senders.clear();
    }
}

// TODO find a better place to put this
pub fn get_file_name(path: PathBuf) -> String {
    let file = match path.file_name() {
        Some(file) => match file.to_str() {
            Some(file) => file.to_string(),
            None => path.display().to_string(),
        },
        None => path.display().to_string(),
    };

    file
}
