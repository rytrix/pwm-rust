pub mod error;
pub mod message;
pub mod prompt;

use crate::gui::error::GuiError;
use crate::state::State;

use std::{path::PathBuf, sync::Arc};

use eframe::egui::{self, Key, Layout, Modifiers, Vec2};
use egui_extras::{Column, TableBuilder};
use log::{error, info, warn};

use pwm_lib::{
    crypt_file::{decrypt_file, encrypt_file},
    random::random_password,
    zeroize::{Zeroize, Zeroizing},
};

pub struct Gui {
    scale: f32,
    update_scale: bool,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    state: Arc<State>,
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            scale: 2.0,
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
                if !self.allowed_to_close {
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
                        o.copied_text.zeroize();
                        o.copied_text = string;
                        *clipboard = None;
                    }
                }
            });


            if let Err(error) = Gui::handle_keybinds(self.state.clone(), ctx) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            let _text_height = egui::TextStyle::Body
                .resolve(ui.style())
                .size
                .max(ui.spacing().interact_size.y);

            if let Err(error) = self.display_header(ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = Gui::display_prompts(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = Gui::display_messages(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error.to_string());
            }

            if let Err(error) = Gui::display_vault(self.state.clone(), ui) {
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
        file
    }

    pub fn save_file_dialog(state: Arc<State>) -> Option<PathBuf> {
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

    fn file_save_setup(state: Arc<State>) -> Option<Zeroizing<String>> {
        match State::contains_vault(state.clone()) {
            Ok(contains) => {
                if !contains {
                    GuiError::display_error_or_print(
                        state.clone(),
                        String::from("No vault opened"),
                    );
                    return None;
                }
            }
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
                return None;
            }
        }

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
                return None;
            }
        };

        Some(password)
    }

    async fn file_save(state: Arc<State>) {
        let password = match Gui::file_save_setup(state.clone()) {
            Some(password) => password,
            None => return,
        };

        let path = match State::get_prev_file(state.clone()) {
            Ok(path) => path,
            Err(error) => {
                GuiError::display_error_or_print(
                    state,
                    format!("File save error: {}", error.to_string()),
                );
                return;
            }
        };

        info!("file_save selected path \"{}\"", path);

        if let Err(error) =
            State::save_vault_to_file(state.clone(), path.as_str(), password.as_bytes()).await
        {
            GuiError::display_error_or_print(state, error.to_string());
        }
    }

    async fn file_save_as(state: Arc<State>) {
        let password = match Gui::file_save_setup(state.clone()) {
            Some(password) => password,
            None => return,
        };

        let path = match Gui::save_file_dialog(state.clone()) {
            Some(path) => path,
            None => return,
        };

        info!("file_save_as selected path \"{}\"", path.display().to_string());

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

    async fn crypt_setup(
        state: Arc<State>,
        prompt: &str,
        prompt2: Option<&str>,
    ) -> Option<(String, Zeroizing<String>)> {
        let file = Self::open_file_dialog(state.clone());
        if let Some(file_path) = file {
            let file = get_file_name(file_path);

            let prompt = format!("{} {}", prompt, file);
            let receiver = match State::add_password_prompt(state.clone(), prompt) {
                Ok(receiver) => receiver,
                Err(_) => return None,
            };

            let password = match receiver.recv() {
                Ok(password) => password,
                Err(error) => {
                    GuiError::display_error_or_print(state.clone(), error.to_string());
                    return None;
                }
            };

            if let Some(prompt2) = prompt2 {
                let prompt = format!("{} {}", prompt2, file);
                let receiver = match State::add_password_prompt(state.clone(), prompt) {
                    Ok(receiver) => receiver,
                    Err(_) => return None,
                };

                let password2 = match receiver.recv() {
                    Ok(pass) => pass,
                    Err(_) => return None,
                };
                if !password.eq(&password2) {
                    return None;
                }
            }

            return Some((String::from(file), password));
        }

        None
    }

    async fn encrypt_file(state: Arc<State>) {
        if let Some((file, password)) = Gui::crypt_setup(
            state.clone(),
            "Enter password to encrypt",
            Some("Confirm password to encrypt"),
        )
        .await
        {
            match encrypt_file(file, None, password.as_bytes()) {
                Ok(()) => (),
                Err(error) => {
                    GuiError::display_error_or_print(state, error.to_string());
                }
            };
        }
    }

    async fn decrypt_file(state: Arc<State>) {
        if let Some((file, password)) =
            Gui::crypt_setup(state.clone(), "Enter password to decrypt", None).await
        {
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

                let password = match random_password(length) {
                    Ok(password) => password,
                    Err(error) => {
                        GuiError::display_error_or_print(state.clone(), error.to_string());
                        return;
                    }
                };

                *clipboard = Some(Zeroizing::new(password));
            }
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
            }
        }
    }

    async fn clear_clipboard(state: Arc<State>) {
        let mut clipboard = match state.clipboard_string.lock() {
            Ok(clipboard) => clipboard,
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.to_string());
                return;
            }
        };

        *clipboard = Some(Zeroizing::new(String::from("0")));
    }

    async fn insert(state: Arc<State>, name: String) {
        if let Err(error) = State::insert(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    async fn insert_from_csv(state: Arc<State>) {
        if let Err(error) = State::insert_from_csv(state.clone()).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    async fn export_to_csv(state: Arc<State>) {
        if let Err(error) = State::export_to_csv(state.clone()).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    async fn rename(state: Arc<State>, name: String) {
        if let Err(error) = State::rename(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    async fn replace(state: Arc<State>, name: String) {
        if let Err(error) = State::replace(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    async fn remove(state: Arc<State>, name: String) {
        if let Err(error) = State::remove(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    async fn get(state: Arc<State>, name: String) {
        if let Err(error) = State::get(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error.to_string());
        }
    }

    fn was_vault_modified(state: Arc<State>) -> bool {
        let vault = match state.vault.lock() {
            Ok(vault) => vault,
            Err(error) => {
                // TODO better error handling?
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
                    ui.close_menu();
                }
                if ui.button("Open").clicked() {
                    tokio::spawn(Gui::file_open(self.state.clone()));
                    ui.close_menu();
                }
                if ui.button("Save").clicked() {
                    tokio::spawn(Gui::file_save(self.state.clone()));
                    ui.close_menu();
                }
                if ui.button("Save As").clicked() {
                    tokio::spawn(Gui::file_save_as(self.state.clone()));
                    ui.close_menu();
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
                    ui.close_menu();
                }
                if ui.button("Decrypt File").clicked() {
                    tokio::spawn(Gui::decrypt_file(self.state.clone()));
                    ui.close_menu();
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
                                    ui.close_menu();
                                }
                                if ui.button("Generate").clicked() {
                                    tokio::spawn(Gui::random_password(self.state.clone()));
                                    ui.close_menu();
                                }
                            });
                        });
                    }
                    Err(error) => {
                        GuiError::display_error_or_print(self.state.clone(), error.to_string());
                    }
                }
            });

            if ui.button("Clear Clipboard").clicked() {
                tokio::spawn(Gui::clear_clipboard(self.state.clone()));
            }
        });

        ui.separator();
        Ok(())
    }

    fn display_prompts(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let mut prompts = state.prompts.lock()?;
        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if prompts.len() <= 0 {
            return Ok(());
        }

        for prompt in prompts.iter_mut() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(prompt.prompt.as_str());
                    let (remove, _) = prompt.prompt_ui(ui);
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
            prompts.remove(i);
        }

        Ok(())
    }

    fn display_messages(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let mut messages = state.messages.lock()?;
        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if messages.len() <= 0 {
            return Ok(());
        }

        for message in messages.iter() {
            if !message.is_complete() {
                message.display(ui);
            } else {
                remove_list.push(count);
            }
            count += 1;
        }

        ui.separator();

        // Remove list goes backwards
        remove_list.reverse();

        for i in remove_list {
            messages.remove(i);
        }

        Ok(())
    }

    fn display_vault(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Ok(()),
        };

        let name = vault.name_buffer.clone();

        ui.horizontal(|ui| {
            ui.heading(name);
            let response = ui.button("Search");
            let popup_id = ui.make_persistent_id("SearchPopupId");

            if response.clicked() {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }
            if ui.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::F)) {
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            }

            egui::popup_below_widget(ui, popup_id, &response, |ui| {
                let mut buffer = match state.search_string.lock() {
                    Ok(buffer) => buffer,
                    Err(error) => {
                        GuiError::display_error_or_print(state.clone(), error.to_string());
                        return ();
                    }
                };
                ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut *buffer))
                    .request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    ui.memory_mut(|mem| mem.close_popup());
                }
            });

            let response = ui.button("Insert");
            let popup_id = ui.make_persistent_id("InsertPopupId");

            if response.clicked() {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }
            if ui.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::I)) {
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            }

            egui::popup_below_widget(ui, popup_id, &response, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [100.0, 20.0],
                        egui::TextEdit::singleline(&mut vault.insert_buffer),
                    );
                    response.request_focus();

                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        tokio::spawn(Gui::insert(state.clone(), vault.insert_buffer.clone()));
                        vault.insert_buffer.clear();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                    if ui.button("Enter").clicked() {
                        tokio::spawn(Gui::insert(state.clone(), vault.insert_buffer.clone()));
                        vault.insert_buffer.clear();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                });
            });

            ui.menu_button("Csv", |ui| {
                if ui.button("Import").clicked() {
                    tokio::spawn(Gui::insert_from_csv(state.clone()));
                    ui.close_menu();
                }

                if ui.button("Export").clicked() {
                    tokio::spawn(Gui::export_to_csv(state.clone()));
                    ui.close_menu();
                }
            });
        });

        ui.separator();

        let list = vault.list_fuzzy_match(state.search_string.lock()?.as_str())?;
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
                                tokio::spawn(Gui::get(state.clone(), name.clone()));
                            }
                            ui.menu_button("Modify", |ui| {
                                if ui.button("Rename").clicked() {
                                    tokio::spawn(Gui::rename(state.clone(), name.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("Replace").clicked() {
                                    tokio::spawn(Gui::replace(state.clone(), name.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("Delete").clicked() {
                                    tokio::spawn(Gui::remove(state.clone(), name.clone()));
                                    ui.close_menu();
                                }
                            });
                            ui.add_space(6.0)
                        });
                    });
                }
            });

        Ok(())
    }

    fn handle_keybinds(state: Arc<State>, ctx: &egui::Context) -> Result<(), GuiError> {
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::N)) {
            tokio::spawn(Gui::file_new(state.clone()));
            info!("File New");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::O)) {
            tokio::spawn(Gui::file_open(state.clone()));
            info!("File Open");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::S)) {
            tokio::spawn(Gui::file_save(state.clone()));
            info!("File Save");
        }
        if ctx.input(|i| {
            i.modifiers
                .matches_exact(Modifiers::CTRL | Modifiers::SHIFT)
                && i.key_pressed(Key::S)
        }) {
            tokio::spawn(Gui::file_save_as(state.clone()));
            info!("File Save as");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::E)) {
            tokio::spawn(Gui::encrypt_file(state.clone()));
            info!("Encrypt File");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::D)) {
            tokio::spawn(Gui::encrypt_file(state.clone()));
            info!("Decrypt File");
        }

        Ok(())
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        let mut senders = self.state.prompts.lock().unwrap();
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
