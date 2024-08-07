pub mod error;
pub mod message;
pub mod prompt;

use crate::config::{default_config, write_config};
use crate::state::State;
use crate::vault::Vault;
use crate::{config::get_config, gui::error::GuiError};

use std::collections::VecDeque;
use std::path::Component;
use std::{path::PathBuf, sync::Arc};

use eframe::egui::{
    self, Button, Color32, Key, Label, Layout, Modifiers, Rounding, Sense, Stroke, Style, Vec2,
};
use eframe::CreationContext;
use egui_extras::{Column, TableBuilder};
use log::{debug, error, info, warn};

use pwm_lib::{
    crypt_file::{decrypt_file, encrypt_file},
    random::random_password,
    zeroize::{Zeroize, Zeroizing},
};

pub struct Gui {
    scale: f32,
    update_scale: bool,

    darkmode: bool,

    // Copies the original value and is modified by the text box in options
    prev_vaults_max_length_text: String,

    // Exit confirmation if a vault was modified
    show_exit_confirmation_dialog: bool,
    allowed_to_close: bool,

    // New vault confirmation if a vault was modified
    show_new_vault_confirmation_dialog: bool,
    create_new_vault_confirmed: bool,

    // Close vault confirmation if a vault was modified
    show_close_vault_confirmation_dialog: bool,
    close_vault_confirmed: bool,

    // Vault insert buffer
    vault_insert_buffer: String,

    state: Arc<State>,
}

impl Gui {
    pub fn new(cc: &CreationContext) -> Self {
        let config = get_config();
        let prev_vaults_json = &config["prev_vaults"];
        let mut prev_vaults = VecDeque::new();
        debug!(
            "prev_vaults_json is array: {} members: {:?}",
            prev_vaults_json.is_array(),
            prev_vaults_json
        );
        for value in prev_vaults_json.members() {
            debug!("prev_vault_member: {:?}", value);
            if let Some(string) = value.as_str() {
                prev_vaults.push_back(string.to_string());
            }
        }
        info!("prev_vaults: {:?}", prev_vaults);

        let max_len = config["prev_vaults_max"].as_usize().unwrap_or(8);

        let pass_len = config["password_length"].as_usize().unwrap_or(32);

        Self {
            scale: config["scale"].as_f32().unwrap_or(1.85),
            update_scale: true,

            darkmode: config["dark"].as_bool().unwrap_or(true),

            prev_vaults_max_length_text: format!("{max_len}"),

            show_exit_confirmation_dialog: false,
            allowed_to_close: false,

            show_new_vault_confirmation_dialog: false,
            create_new_vault_confirmed: false,

            show_close_vault_confirmation_dialog: false,
            close_vault_confirmed: false,

            vault_insert_buffer: String::new(),

            state: Arc::new(State::new(
                cc.egui_ctx.clone(),
                prev_vaults,
                max_len,
                pass_len,
            )),
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.style_mut(|style: &mut Style| {
            //style.visuals.button_frame = false;
            if self.darkmode {
                style.visuals.widgets.noninteractive.weak_bg_fill = Color32::from_gray(30);
                style.visuals.widgets.active.weak_bg_fill = Color32::from_gray(42);
                style.visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(42);
                style.visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(47);
                style.visuals.widgets.open.weak_bg_fill = Color32::from_gray(42);

                style.visuals.widgets.active.bg_stroke = Stroke::NONE;
                style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
                style.visuals.widgets.open.bg_stroke = Stroke::NONE;
            }

            style.visuals.widgets.noninteractive.rounding = Rounding::same(1.5);
            style.visuals.widgets.active.rounding = Rounding::same(1.5);
            style.visuals.widgets.inactive.rounding = Rounding::same(1.5);
            style.visuals.widgets.hovered.rounding = Rounding::same(1.5);
            style.visuals.widgets.open.rounding = Rounding::same(1.5);

            style.spacing.item_spacing = Vec2::new(1.2, 1.2);
            style.spacing.menu_width = 100.0;
        });

        let mut frame = egui::Frame::central_panel(&egui::Style::default())
            .inner_margin(egui::Margin::same(1.0));

        if !self.darkmode {
            frame = frame.fill(Color32::from_gray(240));
        }

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            self.update_color_scheme(ui, ctx);

            if ctx.input(|i| i.viewport().close_requested()) {
                if !self.allowed_to_close {
                    if Gui::was_vault_modified(self.state.clone()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.show_exit_confirmation_dialog = true;
                    }
                }
            }

            if self.show_exit_confirmation_dialog {
                if let Err(error) = self.display_exit_confirmation(ctx) {
                    GuiError::display_error_or_print(self.state.clone(), error);
                }
            }

            if self.show_new_vault_confirmation_dialog {
                if let Err(error) = self.display_new_vault_confirmation(ctx) {
                    GuiError::display_error_or_print(self.state.clone(), error);
                }
            }

            if self.show_close_vault_confirmation_dialog {
                if let Err(error) = self.display_close_vault_confirmation(ctx) {
                    GuiError::display_error_or_print(self.state.clone(), error);
                }
            }

            if self.create_new_vault_confirmed {
                tokio::spawn(Gui::file_new_no_check(self.state.clone()));
                self.create_new_vault_confirmed = false;
            }

            if self.close_vault_confirmed {
                tokio::spawn(Gui::close_vault(self.state.clone()));
                self.close_vault_confirmed = false;
            }

            if self.update_scale {
                ctx.set_pixels_per_point(self.scale);
            }

            // Handle clipboard
            ui.output_mut(|o| {
                if let Ok(mut clipboard) = self.state.clipboard_string.write() {
                    if let Some(result) = &mut *clipboard {
                        o.copied_text.zeroize();
                        o.copied_text = result.to_string();
                        *clipboard = None;
                    }
                }
            });

            if let Err(error) = self.handle_keybinds(ctx) {
                GuiError::display_error_or_print(self.state.clone(), error);
            }

            let _text_height = egui::TextStyle::Body
                .resolve(ui.style())
                .size
                .max(ui.spacing().interact_size.y);

            if let Err(error) = self.display_header(ui) {
                GuiError::display_error_or_print(self.state.clone(), error);
            }

            if let Err(error) = Gui::display_prompts(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error);
            }

            if let Err(error) = Gui::display_messages(self.state.clone(), ui) {
                GuiError::display_error_or_print(self.state.clone(), error);
            }

            if let Err(error) = self.display_vault(ui) {
                GuiError::display_error_or_print(self.state.clone(), error);
            }
        });
    }
}

impl Gui {
    fn reset_settings(&mut self) {
        let defaults = default_config();

        self.scale = defaults["scale"]
            .as_f32()
            .expect("default config does not contain scale");
        self.update_scale = true;
        self.darkmode = defaults["dark"]
            .as_bool()
            .expect("default config does not contain dark");
        self.prev_vaults_max_length_text = defaults["prev_vaults_max"].to_string();

        match self.prev_vaults_max_length_text.parse() {
            Ok(max) => {
                tokio::spawn(State::update_prev_vaults_max_length(
                    self.state.clone(),
                    max,
                ));
            }
            Err(error) => {
                GuiError::display_error_or_print(self.state.clone(), error.into());
            }
        }

        match self.state.password_length.write() {
            Ok(mut pw_len) => {
                *pw_len = defaults["password_length"]
                    .as_usize()
                    .expect("password_length not present in default config")
                    .to_string();
            }
            Err(error) => {
                GuiError::display_error_or_print(self.state.clone(), error.into());
            }
        }
    }

    pub fn open_file_dialog(state: Arc<State>) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new();

        match std::env::current_dir() {
            Ok(path) => {
                dialog = dialog.set_directory(path);
            }
            Err(error) => {
                GuiError::display_error_or_print(
                    state.clone(),
                    format!("Could not open current directory: {}", error.to_string()).into(),
                );
            }
        };

        let file = dialog.pick_file();
        file
    }

    pub fn save_file_dialog(state: Arc<State>) -> Option<PathBuf> {
        let dialog = rfd::FileDialog::new();

        let mut dialog = match state.vault.write() {
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

    async fn file_new_no_check(state: Arc<State>) {
        let error = State::create_vault(state.clone()).await;
        if let Err(error) = error {
            GuiError::display_error_or_print(state, error);
        }
    }

    async fn close_vault(state: Arc<State>) {
        let error = State::close_vault(state.clone()).await;
        if let Err(error) = error {
            GuiError::display_error_or_print(state, error);
        }
    }

    // Calls tokio::spawn internally
    fn file_new(&mut self, state: Arc<State>) {
        if Gui::was_vault_modified(state.clone()) {
            self.show_new_vault_confirmation_dialog = true;
        } else {
            tokio::spawn(Gui::file_new_no_check(state));
        }
    }

    async fn file_open(state: Arc<State>) {
        let error = State::open_vault_from_file_dialog(state.clone()).await;

        if let Err(error) = error {
            GuiError::display_error_or_print(state, error);
        }
    }

    async fn file_open_named(state: Arc<State>, name: String) {
        let error = State::open_vault_from_file(state.clone(), name).await;

        if let Err(error) = error {
            GuiError::display_error_or_print(state, error);
        }
    }

    fn file_save_setup(state: Arc<State>) -> Option<Zeroizing<String>> {
        match State::contains_vault(state.clone()) {
            Ok(contains) => {
                if !contains {
                    GuiError::display_error_or_print(
                        state.clone(),
                        String::from("No vault opened").into(),
                    );
                    return None;
                }
            }
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error);
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
                GuiError::display_error_or_print(state.clone(), error);
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
                GuiError::display_error_or_print(state, error);
                return;
            }
        };

        info!("file_save selected path \"{}\"", path);

        if let Err(error) =
            State::save_vault_to_file(state.clone(), path.as_str(), password.as_bytes()).await
        {
            GuiError::display_error_or_print(state, error);
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

        info!(
            "file_save_as selected path \"{}\"",
            path.display().to_string()
        );

        match State::save_vault_to_file(
            state.clone(),
            path.display().to_string().as_str(),
            password.as_bytes(),
        )
        .await
        {
            Ok(()) => (),
            Err(error) => {
                GuiError::display_error_or_print(state, error.into());
            }
        }
    }

    fn update_color_scheme(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.visuals_mut().dark_mode = self.darkmode;
        if self.darkmode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
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
                    GuiError::display_error_or_print(state.clone(), error.into());
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
                    GuiError::display_error_or_print(state, error.into());
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
                        String::from("Failed to decrypt file, invalid password").into(),
                    );
                }
            };
        }
    }

    async fn random_password(state: Arc<State>) {
        let mut clipboard = match state.clipboard_string.write() {
            Ok(clipboard) => clipboard,
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.into());
                return;
            }
        };

        match state.password_length.write() {
            Ok(password_length) => {
                let length: usize = match password_length.parse() {
                    Ok(length) => length,
                    Err(error) => {
                        GuiError::display_error_or_print(state.clone(), error.into());
                        return;
                    }
                };

                let password = match random_password(length) {
                    Ok(password) => password,
                    Err(error) => {
                        GuiError::display_error_or_print(state.clone(), error.to_string().into());
                        return;
                    }
                };

                *clipboard = Some(Zeroizing::new(password));
                state.egui_ctx.request_repaint();
            }
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.into());
            }
        }
    }

    async fn clear_clipboard(state: Arc<State>) {
        let mut clipboard = match state.clipboard_string.write() {
            Ok(clipboard) => clipboard,
            Err(error) => {
                GuiError::display_error_or_print(state.clone(), error.into());
                return;
            }
        };

        *clipboard = Some(Zeroizing::new(String::from("0")));
    }

    async fn insert(state: Arc<State>, name: String) {
        if let Err(error) = State::insert(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn insert_from_csv(state: Arc<State>) {
        if let Err(error) = State::insert_from_csv(state.clone()).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn export_to_csv(state: Arc<State>) {
        if let Err(error) = State::export_to_csv(state.clone()).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn rename(state: Arc<State>, name: String) {
        if let Err(error) = State::rename(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn replace(state: Arc<State>, name: String) {
        if let Err(error) = State::replace(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn remove(state: Arc<State>, name: String) {
        if let Err(error) = State::remove(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn get(state: Arc<State>, name: String) {
        if let Err(error) = State::get(state.clone(), name).await {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    async fn remove_prev_vault(state: Arc<State>, prev_vault: String) {
        if let Err(error) = State::remove_prev_vault(state.clone(), prev_vault.clone()) {
            GuiError::display_error_or_print(state.clone(), error);
        }
    }

    fn was_vault_modified(state: Arc<State>) -> bool {
        let vault = match state.vault.read() {
            Ok(vault) => vault,
            Err(error) => {
                // TODO better error handling?
                error!("Failed to lock mutex: {}", error.to_string());
                return true;
            }
        };
        if let Some(vault) = &*vault {
            return vault.changed;
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
                                self.show_exit_confirmation_dialog = false;
                                self.allowed_to_close = true;
                                columns[0]
                                    .ctx()
                                    .send_viewport_cmd(egui::ViewportCommand::Close);
                            }

                            if columns[1]
                                .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("No"))
                                .clicked()
                            {
                                self.show_exit_confirmation_dialog = false;
                                self.allowed_to_close = false;
                            }
                        });
                    },
                );
            });
        Ok(())
    }

    fn display_new_vault_confirmation(&mut self, ctx: &egui::Context) -> Result<(), GuiError> {
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
                        ui.label("Create new vault anyways?");

                        ui.columns(2, |columns| {
                            if columns[0]
                                .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("Yes"))
                                .clicked()
                            {
                                self.show_new_vault_confirmation_dialog = false;
                                self.create_new_vault_confirmed = true;
                            }

                            if columns[1]
                                .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("No"))
                                .clicked()
                            {
                                self.show_new_vault_confirmation_dialog = false;
                                self.create_new_vault_confirmed = false;
                            }
                        });
                    },
                );
            });
        Ok(())
    }

    fn display_close_vault_confirmation(&mut self, ctx: &egui::Context) -> Result<(), GuiError> {
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
                        ui.label("Close vault anyways?");

                        ui.columns(2, |columns| {
                            if columns[0]
                                .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("Yes"))
                                .clicked()
                            {
                                self.show_close_vault_confirmation_dialog = false;
                                self.close_vault_confirmed = true;
                            }

                            if columns[1]
                                .add_sized(Vec2::new(15.0, 15.0), egui::Button::new("No"))
                                .clicked()
                            {
                                self.show_close_vault_confirmation_dialog = false;
                                self.close_vault_confirmed = false;
                            }
                        });
                    },
                );
            });
        Ok(())
    }

    fn display_header(&mut self, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let vault_locked = State::is_vault_locked(self.state.clone());

        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add_enabled(vault_locked, Button::new("Create"))
                    .clicked()
                    && vault_locked
                {
                    self.file_new(self.state.clone());
                    ui.close_menu();
                }

                if ui.add_enabled(vault_locked, Button::new("Open")).clicked() && vault_locked {
                    tokio::spawn(Gui::file_open(self.state.clone()));
                    ui.close_menu();
                }
                ui.menu_button("Open Recent", |ui| {
                    if let Err(error) = Gui::display_recent_vaults_loop(
                        self.state.clone(),
                        ui,
                        2,
                        false,
                        vault_locked,
                    ) {
                        GuiError::display_error_or_print(self.state.clone(), error);
                    };
                });

                if ui.add_enabled(vault_locked, Button::new("Save")).clicked() {
                    tokio::spawn(Gui::file_save(self.state.clone()));
                    ui.close_menu();
                }
                if ui
                    .add_enabled(vault_locked, Button::new("Save As"))
                    .clicked()
                {
                    tokio::spawn(Gui::file_save_as(self.state.clone()));
                    ui.close_menu();
                }
            });

            ui.menu_button("Options", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Ui Scale");
                    ui.add_space(12.0);
                    ui.checkbox(&mut self.darkmode, "Darkmode");
                });
                if !ui
                    .add(egui::Slider::new(&mut self.scale, 1.0..=3.0))
                    .dragged()
                {
                    self.update_scale = true;
                } else {
                    self.update_scale = false;
                };

                ui.horizontal(|ui| {
                    ui.label("Max Recent Vaults");
                    let response = ui.add_sized(
                        [30.0, 20.0],
                        egui::TextEdit::singleline(&mut self.prev_vaults_max_length_text),
                    );
                    if response.changed() {
                        match self.prev_vaults_max_length_text.parse() {
                            Ok(max) => {
                                tokio::spawn(State::update_prev_vaults_max_length(
                                    self.state.clone(),
                                    max,
                                ));
                            }
                            Err(error) => {
                                GuiError::display_error_or_print(self.state.clone(), error.into());
                            }
                        };
                    }
                });

                if ui.button("Reset to Defaults").clicked() {
                    self.reset_settings();
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
                match self.state.password_length.write() {
                    Ok(mut password_length) => {
                        ui.menu_button("Password Generation", |ui| {
                            ui.label("Password Length");
                            ui.horizontal(|ui| {
                                let response = ui.add_sized(
                                    [40.0, 20.0],
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
                        GuiError::display_error_or_print(self.state.clone(), error.into());
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
        let mut prompts = state.prompts.write()?;
        let mut count = 0;
        let mut remove_list = VecDeque::<usize>::new();

        if prompts.len() <= 0 {
            return Ok(());
        }

        for prompt in prompts.iter_mut() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(prompt.prompt.as_str());
                    let (remove, _) = prompt.prompt_ui(ui);
                    if remove {
                        remove_list.push_front(count);
                    }
                });
            });

            count += 1;
        }

        ui.separator();

        for i in remove_list {
            prompts.remove(i);
        }

        Ok(())
    }

    fn display_messages(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let mut messages = state.messages.write()?;
        let mut count = 0;
        let mut remove_list = VecDeque::<usize>::new();

        if messages.len() <= 0 {
            return Ok(());
        }

        for message in messages.iter() {
            if !message.is_complete() {
                message.display(ui);
            } else {
                remove_list.push_front(count);
            }
            count += 1;
        }

        ui.separator();

        for i in remove_list {
            messages.remove(i);
        }

        Ok(())
    }

    fn display_recent_vaults_loop(
        state: Arc<State>,
        ui: &mut egui::Ui,
        path_len: usize,
        button_mode: bool,
        vault_locked: bool,
    ) -> Result<(), GuiError> {
        let prev_vaults = state.prev_vaults.read()?;
        for prev_vault in prev_vaults.iter() {
            ui.horizontal(|ui| {
                if button_mode {
                    let state_id = ui.id().with(format!("show_full_path_{}", prev_vault));
                    let mut show_full =
                        ui.data_mut(|d| d.get_temp::<bool>(state_id).unwrap_or(false));

                    ui.add_space(3.0);
                    if show_full {
                        if ui
                            .add(Label::new(prev_vault).sense(Sense::click()))
                            .clicked()
                        {
                            show_full = false;
                        }
                    } else {
                        if ui
                            .add(
                                Label::new(get_file_path_back_count(prev_vault.into(), path_len))
                                    .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            show_full = true;
                        }
                    };

                    ui.data_mut(|d| d.insert_temp(state_id, show_full));

                    ui.add_space(6.0);
                    if ui.button("Open").clicked() {
                        tokio::spawn(Gui::file_open_named(state.clone(), prev_vault.clone()));
                    };

                    if ui.button("Forget").clicked() {
                        tokio::spawn(Gui::remove_prev_vault(state.clone(), prev_vault.clone()));
                    };
                } else {
                    if ui
                        .add_enabled(
                            vault_locked,
                            Button::new(get_file_path_back_count(prev_vault.into(), path_len)),
                        )
                        .clicked()
                    {
                        tokio::spawn(Gui::file_open_named(state.clone(), prev_vault.clone()));
                    }
                }
            });
            ui.separator();
        }

        Ok(())
    }

    fn display_recent_vaults(state: Arc<State>, ui: &mut egui::Ui) -> Result<(), GuiError> {
        ui.horizontal(|ui| {
            ui.add_space(3.0);
            ui.heading("Recent Vaults");
        });
        ui.separator();
        Gui::display_recent_vaults_loop(state, ui, 3, true, false)?;

        Ok(())
    }

    fn display_vault(&mut self, ui: &mut egui::Ui) -> Result<(), GuiError> {
        let vault_name_buffer_state_id = ui.id().with("prev_vault_name_buffer");

        // Doesn't wait and displays updating vault if it is not available
        let locked_vault_guard = match self.state.vault.try_write() {
            Ok(vault) => Some(vault),
            Err(_error) => None,
        };
        let mut locked_vault;

        let mut vault: Option<&mut Vault> = None;
        if let Some(locked_vault_internal) = locked_vault_guard {
            locked_vault = locked_vault_internal;
            if let Some(vault_ref) = locked_vault.as_mut() {
                let name = vault_ref.name_buffer.clone();
                ui.data_mut(|d| d.insert_temp(vault_name_buffer_state_id, name));
                vault = Some(vault_ref);
            } else {
                return Gui::display_recent_vaults(self.state.clone(), ui);
            }
        }

        let vault_locked = if let Some(_vault) = vault.as_mut() {
            true
        } else {
            false
        };

        let name = ui.data_mut(|d| {
            d.get_temp::<String>(vault_name_buffer_state_id)
                .unwrap_or(String::from("Unknown Name"))
        });

        if !vault_locked {
            ui.heading("Updating Vault");
        }

        ui.horizontal(|ui| {
            ui.add_space(3.0);
            ui.heading(name);
            ui.add_space(6.0);

            let response = ui.button("Search");
            let popup_id = ui.make_persistent_id("SearchPopupId");

            if response.clicked() {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }
            if ui.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::F)) {
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            }

            egui::popup_below_widget(ui, popup_id, &response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui: &mut egui::Ui| {
                let mut buffer = match self.state.search_string.write() {
                    Ok(buffer) => buffer,
                    Err(error) => {
                        GuiError::display_error_or_print(self.state.clone(), error.into());
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

            egui::popup_below_widget(ui, popup_id, &response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [100.0, 20.0],
                        egui::TextEdit::singleline(&mut self.vault_insert_buffer),
                    );
                    response.request_focus();

                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) && vault_locked {
                        tokio::spawn(Gui::insert(
                            self.state.clone(),
                            self.vault_insert_buffer.clone(),
                        ));
                        self.vault_insert_buffer.clear();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                    if ui.add_enabled(vault_locked, Button::new("Enter")).clicked() && vault_locked
                    {
                        tokio::spawn(Gui::insert(
                            self.state.clone(),
                            self.vault_insert_buffer.clone(),
                        ));
                        self.vault_insert_buffer.clear();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                });
            });

            if vault_locked {
                // I repeat vault_locked checks for safety if I modify this code later
                // I don't want unchecked out of order modificaitons to ever ever happen
                ui.menu_button("Csv", |ui| {
                    if ui
                        .add_enabled(vault_locked, Button::new("Import"))
                        .clicked()
                        && vault_locked
                    {
                        tokio::spawn(Gui::insert_from_csv(self.state.clone()));
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(vault_locked, Button::new("Export"))
                        .clicked()
                        && vault_locked
                    {
                        tokio::spawn(Gui::export_to_csv(self.state.clone()));
                        ui.close_menu();
                    }
                });
            } else {
                ui.add_enabled(vault_locked, Button::new("Csv"));
            }

            if ui.add_enabled(vault_locked, Button::new("Close")).clicked() && vault_locked {
                if let Some(vault) = vault.as_mut() {
                    if vault.changed {
                        self.show_close_vault_confirmation_dialog = true;
                    } else {
                        tokio::spawn(Gui::close_vault(self.state.clone()));
                    }
                }
            };
        });

        ui.separator();

        let vault_list_buffer_state_id = ui.id().with("prev_vault_list_buffer");

        let list = if let Some(vault) = vault.as_mut() {
            let list = vault.list_fuzzy_match(self.state.search_string.write()?.as_str())?;
            ui.data_mut(|d| d.insert_temp(vault_list_buffer_state_id, list.clone()));
            list.clone()
        } else {
            let list = ui.data_mut(|d| {
                d.get_temp::<Vec<String>>(vault_list_buffer_state_id)
                    .unwrap_or(Vec::new())
            });
            list
        };

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
                    ui.add_space(3.0);
                    ui.strong("Username");
                });
                header.col(|ui| {
                    ui.add_space(6.0);
                    ui.strong("Password");
                });
            })
            .body(|mut body| {
                for row_index in 0..list.len() {
                    let row_height = 30.0;
                    body.row(row_height, |mut row| {
                        let name = &list[row_index];
                        row.col(|ui| {
                            ui.add_space(3.0);
                            ui.label(format!("{}", name.clone()));
                        });
                        row.col(|ui| {
                            ui.add_space(6.0);
                            if ui.add_enabled(vault_locked, Button::new("Get")).clicked()
                                && vault_locked
                            {
                                tokio::spawn(Gui::get(self.state.clone(), name.clone()));
                            }
                            ui.add_space(3.0);
                            if vault_locked {
                                ui.menu_button("Modify", |ui| {
                                    if ui
                                        .add_enabled(vault_locked, Button::new("Rename"))
                                        .clicked()
                                        && vault_locked
                                    {
                                        tokio::spawn(Gui::rename(self.state.clone(), name.clone()));
                                        ui.close_menu();
                                    }
                                    if ui.add_enabled(vault_locked, Button::new("Edit")).clicked()
                                        && vault_locked
                                    {
                                        tokio::spawn(Gui::replace(
                                            self.state.clone(),
                                            name.clone(),
                                        ));
                                        ui.close_menu();
                                    }
                                    if ui
                                        .add_enabled(vault_locked, Button::new("Delete"))
                                        .clicked()
                                        && vault_locked
                                    {
                                        tokio::spawn(Gui::remove(self.state.clone(), name.clone()));
                                        ui.close_menu();
                                    }
                                });
                            } else {
                                ui.add_enabled(vault_locked, Button::new("Modify"));
                            }
                            ui.add_space(6.0)
                        });
                    });
                }
            });

        Ok(())
    }

    fn handle_keybinds(&mut self, ctx: &egui::Context) -> Result<(), GuiError> {
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::N)) {
            self.file_new(self.state.clone());
            info!("File New");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::O)) {
            tokio::spawn(Gui::file_open(self.state.clone()));
            info!("File Open");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::S)) {
            tokio::spawn(Gui::file_save(self.state.clone()));
            info!("File Save");
        }
        if ctx.input(|i| {
            i.modifiers
                .matches_exact(Modifiers::CTRL | Modifiers::SHIFT)
                && i.key_pressed(Key::S)
        }) {
            tokio::spawn(Gui::file_save_as(self.state.clone()));
            info!("File Save as");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::E)) {
            tokio::spawn(Gui::encrypt_file(self.state.clone()));
            info!("Encrypt File");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::D)) {
            tokio::spawn(Gui::encrypt_file(self.state.clone()));
            info!("Decrypt File");
        }
        if ctx.input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_pressed(Key::Q)) {
            let none = None;
            let vault_real;
            let vault = match self.state.vault.read() {
                Ok(vault) => {
                    vault_real = vault;
                    &*vault_real
                }
                Err(error) => {
                    GuiError::display_error_or_print(self.state.clone(), error.into());
                    &none
                }
            };
            if let Some(vault) = vault {
                if vault.changed {
                    self.show_close_vault_confirmation_dialog = true;
                } else {
                    tokio::spawn(Gui::close_vault(self.state.clone()));
                }
            }
            info!("Close Vault");
        }

        Ok(())
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        match self.state.prev_vaults.read() {
            Ok(prev_vaults) => {
                let prev_vaults_vec: Vec<String> =
                    prev_vaults.iter().map(|value| value.clone()).collect();

                let max_length = match self.state.prev_vaults_max_length.read() {
                    Ok(max_length) => *max_length,
                    Err(error) => {
                        warn!("State::prev_vaults_max_length was unable to be unlocked defaulting to 8: {}", error.to_string());
                        8
                    }
                };

                let slice_len = if prev_vaults_vec.len() < max_length {
                    prev_vaults_vec.len()
                } else {
                    max_length
                };

                let password_len = match self.state.password_length.read() {
                    Ok(password_length) => match password_length.parse() {
                        Ok(password_len) => password_len,
                        Err(_error) => {
                            warn!("failed to parse password_length string, defaulting to 32");
                            32
                        }
                    },
                    Err(error) => {
                        warn!("State::password_length was unable to be unlocked defaulting to \"32\": {}", error.to_string());
                        32
                    }
                };

                let config = json::object! {
                    dark: self.darkmode,
                    scale: self.scale,
                    prev_vaults: prev_vaults_vec[0..slice_len],
                    prev_vaults_max: max_length,
                    password_length: password_len,
                };

                write_config(config);
            }
            Err(error) => warn!("Failed to save config: {}", error.to_string()),
        };

        let mut senders = self.state.prompts.write().unwrap();
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

pub fn get_file_path_back_count(path: PathBuf, back_count: usize) -> String {
    if path.components().count() <= back_count {
        return path.display().to_string();
    }

    let components = path.components().collect::<Vec<Component>>();
    let result = components[components.len() - back_count..]
        .iter()
        .collect::<PathBuf>();

    result.display().to_string()
}
