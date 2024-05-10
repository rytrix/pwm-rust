pub mod error;

use crate::{password::password_ui, state::State};

use std::{path::PathBuf, sync::Arc};

use eframe::egui::{self, Layout, Vec2};
use egui_extras::{Column, TableBuilder};
use log::{error, info, warn};

use pwm_lib::{
    crypt_file::{decrypt_file, encrypt_file},
    random::random_password,
    zeroize::Zeroizing,
};

use crate::gui::error::GuiError;

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

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ctx.input(|i| i.viewport().close_requested()) {
                if self.allowed_to_close {
                    // do nothing - we will close
                } else {
                    if Gui::was_vault_modified(self.state.clone()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.show_confirmation_dialog = true;
                    }
                }
            }

            if self.show_confirmation_dialog {
                if let Err(error) = self.display_exit_confirmation(ctx) {
                    GuiError::display_error_or_print(self.state.clone(), error.to_string());
                }
            }

            if self.update_scale {
                ctx.set_pixels_per_point(self.scale);
            }

            // Handle clipboard
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

            if let Err(error) = self.display_header(ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = Gui::display_password_prompts(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = Gui::display_errors(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = Gui::display_vault(self.state.clone(), ui) {
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

    fn display_exit_confirmation(&mut self, ctx: &egui::Context) -> Result<(), GuiError> {
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
        Ok(())
    }

    fn display_header(&mut self, ui: &mut egui::Ui) -> Result<(), GuiError> {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Create").clicked() {
                    tokio::spawn(Gui::file_new(self.state.clone()));
                }
                if ui.button("Open").clicked() {
                    tokio::spawn(Gui::file_open(self.state.clone()));
                }
                if ui.button("Save").clicked() {
                    tokio::spawn(Gui::file_save(self.state.clone()));
                }
                if ui.button("Save As").clicked() {
                    tokio::spawn(Gui::file_save_as(self.state.clone()));
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
                    tokio::spawn(Gui::encrypt_file(self.state.clone()));
                }
                if ui.button("Decrypt File").clicked() {
                    tokio::spawn(Gui::decrypt_file(self.state.clone()));
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
                                    tokio::spawn(Gui::random_password(self.state.clone()));
                                }
                                if ui.button("Generate").clicked() {
                                    tokio::spawn(Gui::random_password(self.state.clone()));
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
        Ok(())
    }

    fn display_password_prompts(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let mut passwords = state.password.lock()?;
        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if passwords.len() <= 0 {
            return Ok(());
        }

        for (prompt, password, sender) in passwords.iter_mut() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(prompt.as_str());
                    let (remove, _) = password_ui(ui, (password, sender));
                    if remove {
                        remove_list.push(count);
                    }
                });
            });

            count += 1;
        }

        ui.separator();

        // Remove list goes backwards
        remove_list.reverse();

        for i in remove_list {
            passwords.remove(i);
        }

        Ok(())
    }

    fn display_errors(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let mut errors = state.errors.lock()?;
        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if errors.len() <= 0 {
            return Ok(());
        }

        for (error, timer) in errors.iter() {
            if !timer.is_complete() {
                ui.horizontal(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Error");
                        ui.label(error.as_str());
                    });
                });
            } else {
                remove_list.push(count);
            }
            count += 1;
        }

        ui.separator();

        // Remove list goes backwards
        remove_list.reverse();

        for i in remove_list {
            errors.remove(i);
        }

        Ok(())
    }

    fn display_vault(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let list: Vec<String>;
        let name: String;

        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Ok(()),
        };

        list = vault.list_fuzzy_match(state.search_string.lock()?.as_str())?;
        name = vault.name_buffer.clone();

        ui.horizontal(|ui| {
            ui.heading(name);
            ui.menu_button("Search", |ui| {
                let mut buffer = match state.search_string.lock() {
                    Ok(buffer) => buffer,
                    Err(error) => {
                        GuiError::display_error_or_print(state.clone(), error.to_string());
                        return ();
                    }
                };
                let _response =
                    ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut *buffer));
            });
            ui.menu_button("Insert", |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [100.0, 20.0],
                        egui::TextEdit::singleline(&mut vault.insert_buffer),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        tokio::spawn(State::insert(state.clone(), vault.insert_buffer.clone()));
                        vault.insert_buffer.clear();
                    }
                    if ui.button("Enter").clicked() {
                        tokio::spawn(State::insert(state.clone(), vault.insert_buffer.clone()));
                        vault.insert_buffer.clear();
                    }
                });
            });
            ui.menu_button("Csv", |ui| {
                if ui.button("Import").clicked() {
                    tokio::spawn(State::insert_from_csv(state.clone()));
                }

                if ui.button("Export").clicked() {
                    tokio::spawn(State::export_to_csv(state.clone()));
                }
            });
        });

        ui.separator();

        let builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::remainder())
            .column(Column::auto())
            .min_scrolled_height(0.0);

        builder
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Username");
                });
                header.col(|ui| {
                    ui.strong("Password");
                });
            })
            .body(|mut body| {
                for row_index in 0..list.len() {
                    let row_height = 30.0;
                    body.row(row_height, |mut row| {
                        let name = &list[row_index];
                        row.col(|ui| {
                            ui.label(format!("{}", name.clone()));
                        });
                        row.col(|ui| {
                            if ui.button("Get").clicked() {
                                tokio::spawn(State::get(state.clone(), name.clone()));
                            }
                            if ui.button("Delete").clicked() {
                                tokio::spawn(State::remove(state.clone(), name.clone()));
                            }
                            ui.add_space(4.0)
                        });
                    });
                }
            });

        Ok(())
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
