use crate::Gui;
use eframe::egui;

impl Gui {
    #[allow(clippy::ptr_arg)] // false positive
    fn password_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
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

            if ui.button("Enter").clicked() {
                self.password.clear();
            }

            // Show the password field:
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(&mut self.password).password(!show_plaintext),
            );
        });

        ui.data_mut(|d| d.insert_temp(state_id, show_plaintext));

        result.response
    }

    // A wrapper that allows the more idiomatic usage pattern: `ui.add(…)`
    pub fn password(&mut self) -> impl egui::Widget + '_ {
        move |ui: &mut egui::Ui| self.password_ui(ui)
    }
}
