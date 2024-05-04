use eframe::egui::Ui;

use crate::password::password_ui;
use crate::GuiError;
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
    pub fn add_password_prompt(
        state: Arc<State>,
        prompt: String,
    ) -> Result<Receiver<String>, GuiError> {
        let (sender, receiver) = channel();

        let mut vec = match state.password.lock() {
            Ok(vec) => vec,
            Err(error) => return Err(GuiError::LockFail(error.to_string())),
        };
        vec.push((prompt, String::new(), sender));

        return Ok(receiver);
    }

    pub fn display_password_prompts(state: Arc<State>, ui: &mut Ui) -> Result<(), GuiError> {
        let mut passwords = match state.password.lock() {
            Ok(passwords) => passwords,
            Err(error) => return Err(GuiError::LockFail(error.to_string())),
        };

        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if passwords.len() <= 0 {
            return Ok(());
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

        Ok(())
    }

    pub fn add_error(state: Arc<State>, error: (String, Timer)) -> Result<(), GuiError> {
        let mut errors = match state.errors.lock() {
            Ok(errors) => errors,
            Err(error) => return Err(GuiError::LockFail(error.to_string()))
        };
        errors.push(error);

        Ok(())
    }

    pub fn display_errors(state: Arc<State>, ui: &mut Ui) -> Result<(), GuiError> {
        let mut errors = match state.errors.lock() {
            Ok(errors) => errors,
            Err(error) => return Err(GuiError::LockFail(error.to_string()))
        };

        let mut count = 0;
        let mut remove_list = Vec::<usize>::new();

        if errors.len() <= 0 {
            return Ok(());
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

        Ok(())
    }
}
