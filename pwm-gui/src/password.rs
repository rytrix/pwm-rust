use std::{borrow::BorrowMut, sync::mpsc::Sender};

use eframe::egui;
use pwm_lib::zeroize::Zeroizing;

#[allow(clippy::ptr_arg)] // false positive
pub fn password_ui(
    ui: &mut egui::Ui,
    password: (&mut Zeroizing<String>, &mut Sender<Zeroizing<String>>),
) -> (bool, egui::Response) {
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
            password.1.send(password.0.clone()).unwrap();
            remove = true;
        }

        // Show the password field:
        let buffer: &mut String = password.0.borrow_mut();
        let response = ui.add_sized(
            ui.available_size(),
            egui::TextEdit::singleline(buffer).password(!show_plaintext),
        );

        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            password.1.send(password.0.clone()).unwrap();
            remove = true;
        }
    });

    ui.data_mut(|d| d.insert_temp(state_id, show_plaintext));

    (remove, result.response)
}
