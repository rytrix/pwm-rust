use eframe::egui::Ui;

use crate::password::password_ui;
use crate::Timer;
use crate::Vault;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;

pub struct State {
    pub prev_file: Mutex<Option<String>>,
    pub errors: Mutex<Vec<(String, Timer)>>,
    // Prompt, Password, Sender
    pub password: Mutex<Vec<(String, String, Sender<String>)>>,
    pub vault: Mutex<Option<Vault>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            prev_file: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
            password: Mutex::new(Vec::new()),
            vault: Mutex::new(None),
        }
    }
}

impl State {
    pub fn add_password_prompt(state: Arc<State>, prompt: String) -> Receiver<String> {
        let (sender, receiver) = channel();

        let mut vec = state.password.lock().unwrap();
        vec.push((prompt, String::new(), sender));

        return receiver;
    }

    pub fn display_password_prompts(state: Arc<State>, ui: &mut Ui) {
        let mut passwords = state.password.lock().unwrap();

        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if passwords.len() <= 0 {
            return;
        }

        ui.separator();

        for (prompt, password, sender) in passwords.iter_mut() {
            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
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
    }

    pub fn add_error(state: Arc<State>, error: (String, Timer)) {
        let mut errors = state.errors.lock().unwrap();
        errors.push(error);
    }

    pub fn display_errors(state: Arc<State>, ui: &mut Ui) {
        let mut errors = state.errors.lock().unwrap();

        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if errors.len() <= 0 {
            return;
        }

        ui.separator();

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
    }
}
