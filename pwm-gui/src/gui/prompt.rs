use std::{borrow::BorrowMut, sync::mpsc::Sender};

use eframe::egui;
use pwm_lib::zeroize::Zeroizing;

pub struct Prompt {
    pub prompt: String,
    pub response: Zeroizing<String>,
    pub sender: Sender<Zeroizing<String>>,
    password_prompt: bool,
}

impl Prompt {
    pub fn new(
        prompt: String,
        response: Zeroizing<String>,
        sender: Sender<Zeroizing<String>>,
        password_prompt: bool,
    ) -> Self {
        Self {
            prompt,
            response,
            sender,
            password_prompt,
        }
    }

    fn prompt_ui_internal(&mut self, ui: &mut egui::Ui) -> (bool, egui::Response) {
        let mut remove = false;

        let result = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Cancel").clicked() {
                remove = true;
            }

            if ui.button("Enter").clicked() {
                self.sender.send(self.response.clone()).unwrap();
                remove = true;
            }

            let buffer: &mut String = self.response.borrow_mut();
            let response = ui.add_sized(ui.available_size(), egui::TextEdit::singleline(buffer));

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.sender.send(self.response.clone()).unwrap();
                remove = true;
            }
        });

        (remove, result.response)
    }

    fn password_ui_internal(&mut self, ui: &mut egui::Ui) -> (bool, egui::Response) {
        let mut remove = false;
        // Generate an id for the state
        let state_id = ui.id().with("show_plaintext");

        let mut show_plaintext = ui.data_mut(|d| d.get_temp::<bool>(state_id).unwrap_or(false));

        let result = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Toggle the `show_plaintext` bool with a button:
            let response = ui
                .add(egui::SelectableLabel::new(show_plaintext, "👁"))
                .on_hover_text("Show/hide password");

            if response.clicked() {
                show_plaintext = !show_plaintext;
            }

            if ui.button("Cancel").clicked() {
                remove = true;
            }

            if ui.button("Enter").clicked() {
                self.sender.send(self.response.clone()).unwrap();
                remove = true;
            }

            // Show the password field:
            let buffer: &mut String = self.response.borrow_mut();
            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(buffer).password(!show_plaintext),
            );

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.sender.send(self.response.clone()).unwrap();
                remove = true;
            }
        });

        ui.data_mut(|d| d.insert_temp(state_id, show_plaintext));

        (remove, result.response)
    }

    pub fn prompt_ui(&mut self, ui: &mut egui::Ui) -> (bool , egui::Response) {
        if self.password_prompt {
            return self.password_ui_internal(ui)
        } else {
            return self.prompt_ui_internal(ui)
        }
    }
}
