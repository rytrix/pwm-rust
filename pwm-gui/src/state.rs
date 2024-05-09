use eframe::egui;
use eframe::egui::Ui;
use egui_extras::{Column, TableBuilder};
use pwm_lib::zeroize::Zeroizing;

use crate::gui::{get_file_name, Gui, GuiError};
use crate::password::password_ui;
use crate::timer::Timer;
use crate::vault::Vault;

use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;

pub struct State {
    pub prev_file: Mutex<Option<String>>,
    pub errors: Mutex<Vec<(String, Timer)>>,
    // Prompt, Password, Sender
    pub password: Mutex<Vec<(String, Zeroizing<String>, Sender<Zeroizing<String>>)>>,
    pub vault: Mutex<Option<Vault>>,
    pub clipboard_string: Mutex<Option<Zeroizing<String>>>,
    pub search_string: Mutex<String>,
    pub password_length: Mutex<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            prev_file: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
            password: Mutex::new(Vec::new()),
            vault: Mutex::new(None),
            clipboard_string: Mutex::new(None),
            search_string: Mutex::new(String::new()),
            password_length: Mutex::new(String::new()),
        }
    }
}

impl State {
    pub async fn create_vault(state: Arc<State>) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(
            state.clone(),
            String::from("Enter new vault's master password"),
        )?;
        let password = receiver.recv()?;

        let mut vault = state.vault.lock()?;
        *vault = match Vault::new("New Vault", password.as_bytes()) {
            Ok(vault) => Some(vault),
            Err(error) => return Err(GuiError::DatabaseError(error.to_string())),
        };

        Ok(())
    }

    pub async fn open_vault_from_file(state: Arc<State>) -> Result<(), GuiError> {
        let file = match Gui::open_file_dialog(state.clone(), true) {
            Some(file) => file.display().to_string(),
            None => return Err(GuiError::NoFile),
        };

        let receiver = Self::add_password_prompt(
            state.clone(),
            String::from("Enter new vault's master password"),
        )?;
        let password = receiver.recv()?;

        let mut vault = state.vault.lock()?;
        *vault = match Vault::new_from_file(file.as_str(), password.as_bytes()) {
            Ok(vault) => Some(vault),
            Err(error) => return Err(GuiError::DatabaseError(error.to_string())),
        };

        Ok(())
    }

    pub async fn save_vault_to_file(
        state: Arc<State>,
        path: &str,
        password: &[u8],
    ) -> Result<(), GuiError> {
        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.name_buffer = get_file_name(Path::new(path).to_path_buf());

        vault.serialize_to_file(&path, password)?;
        Ok(())
    }

    pub fn display_vault(state: Arc<State>, ui: &mut Ui) -> Result<(), GuiError> {
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
                        return ()
                    }
                };
                let _response = ui.add_sized(
                    [100.0, 20.0],
                    egui::TextEdit::singleline(&mut *buffer),
                );
            });
            ui.menu_button("Insert", |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [100.0, 20.0],
                        egui::TextEdit::singleline(&mut vault.insert_buffer),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        tokio::spawn(Self::insert(state.clone(), vault.insert_buffer.clone()));
                        vault.insert_buffer.clear();
                    }
                    if ui.button("Enter").clicked() {
                        tokio::spawn(Self::insert(state.clone(), vault.insert_buffer.clone()));
                        vault.insert_buffer.clear();
                    }
                });
            });
            ui.menu_button("Csv", |ui| {
                if ui.button("Import").clicked() {
                    tokio::spawn(Self::insert_from_csv(state.clone()));
                }

                if ui.button("Export").clicked() {
                    tokio::spawn(Self::export_to_csv(state.clone()));
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
                // header.col(|ui| {
                //     ui.strong("Row");
                // });
                header.col(|ui| {
                    ui.strong("Key");
                });
                header.col(|ui| {
                    ui.strong("Data");
                });
            })
            .body(|mut body| {
                for row_index in 0..list.len() {
                    let row_height = 30.0;
                    body.row(row_height, |mut row| {
                        let name = &list[row_index];

                        // row.col(|ui| {
                        //     ui.label(row_index.to_string());
                        // });
                        row.col(|ui| {
                            ui.label(format!("{}", name.clone()));
                        });
                        row.col(|ui| {
                            if ui.button("Get").clicked() {
                                tokio::spawn(Self::get(state.clone(), name.clone()));
                            }
                            if ui.button("Delete").clicked() {
                                tokio::spawn(Self::remove(state.clone(), name.clone()));
                            }
                        });
                    });
                }
            });

        Ok(())
    }

    pub async fn insert(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;

        let password = receiver.recv()?;
        let receiver =
            Self::add_password_prompt(state.clone(), format!("Enter entry for {}", name))?;

        let data = receiver.recv()?;

        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.insert(&name, data.as_bytes(), password.as_bytes())?;
        Ok(())
    }

    pub async fn insert_from_csv(state: Arc<State>) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let file = match Gui::open_file_dialog(state.clone(), false) {
            Some(file) => file,
            None => return Err(GuiError::NoFile),
        };

        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.insert_from_csv(file.display().to_string().as_str(), password.as_bytes())?;
        Ok(())
    }

    pub async fn export_to_csv(state: Arc<State>) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let file = match Gui::save_file_dialog(state.clone(), false) {
            Some(file) => file,
            None => return Err(GuiError::NoFile),
        };

        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.export_to_csv(file.display().to_string().as_str(), password.as_bytes())?;
        Ok(())
    }

    pub async fn remove(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let mut vault = state.vault.lock()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.remove(&name, password.as_bytes())?;
        Ok(())
    }

    pub async fn get(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let vault = state.vault.lock()?;
        let vault = match &*vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        let result = vault.get(&name, password.as_bytes())?;

        use std::str;
        let result = match str::from_utf8(result.as_ref()) {
            Ok(result) => result,
            Err(error) => {
                return Err(GuiError::Utf8Fail(format!(
                    "Invalid UTF-8 sequence: {}",
                    error
                )))
            }
        };

        let mut string = state.clipboard_string.lock()?;
        *string = Some(Zeroizing::new(result.to_string()));

        Ok(())
    }

    pub fn add_password_prompt(
        state: Arc<State>,
        prompt: String,
    ) -> Result<Receiver<Zeroizing<String>>, GuiError> {
        let (sender, receiver) = channel();

        let mut vec = state.password.lock()?;
        vec.push((prompt, Zeroizing::new(String::new()), sender));

        return Ok(receiver);
    }

    pub fn display_password_prompts(state: Arc<State>, ui: &mut Ui) -> Result<(), GuiError> {
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

    pub fn add_error(state: Arc<State>, error: (String, Timer)) -> Result<(), GuiError> {
        let mut errors = state.errors.lock()?;
        errors.push(error);

        Ok(())
    }

    pub fn display_errors(state: Arc<State>, ui: &mut Ui) -> Result<(), GuiError> {
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
}
