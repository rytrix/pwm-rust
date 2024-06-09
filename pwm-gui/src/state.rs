use eframe::egui;
use pwm_lib::zeroize::Zeroizing;

use crate::gui::message::Message;
use crate::gui::{error::GuiError, get_file_name, Gui};
use crate::vault::Vault;

use std::collections::VecDeque;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::sync::RwLock;

use crate::gui::prompt::Prompt;

pub struct State {
    pub messages: RwLock<Vec<Message>>,
    // Prompt, Password, Sender
    pub prompts: RwLock<Vec<Prompt>>,
    pub vault: RwLock<Option<Vault>>,
    pub clipboard_string: RwLock<Option<Zeroizing<String>>>,
    pub search_string: RwLock<String>,
    pub password_length: RwLock<String>,
    pub prev_vaults: RwLock<VecDeque<String>>,
    pub prev_vaults_max_length: RwLock<usize>,
    pub egui_ctx: egui::Context,
}

impl State {
    pub fn new(ctx: egui::Context, prev_vaults: VecDeque<String>, prev_vaults_max_length: usize, password_length: usize) -> Self {
        Self {
            messages: RwLock::new(Vec::new()),
            prompts: RwLock::new(Vec::new()),
            vault: RwLock::new(None),
            clipboard_string: RwLock::new(None),
            search_string: RwLock::new(String::new()),
            password_length: RwLock::new(format!("{}", password_length)),
            prev_vaults: RwLock::new(prev_vaults),
            prev_vaults_max_length: RwLock::new(prev_vaults_max_length),
            egui_ctx: ctx
        }
    }

    pub async fn create_vault(state: Arc<State>) -> Result<(), GuiError> {
        let password = State::add_confirmation_password_prompt(
            state.clone(),
            String::from("Enter new vault's master password"),
            String::from("Confirm new vault's master password"),
        )?;

        let mut vault = state.vault.write()?;
        *vault = match Vault::new("New Vault", password.as_bytes()) {
            Ok(vault) => Some(vault),
            Err(error) => return Err(GuiError::DatabaseError(error.to_string())),
        };

        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn close_vault(state: Arc<State>) -> Result<(), GuiError> {
        let mut vault = state.vault.write()?;
        *vault = None;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn open_vault_from_file(state: Arc<State>, file: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(
            state.clone(),
            format!(
                "Enter {}'s master password",
                get_file_name(file.clone().into())
            ),
        )?;
        let password = receiver.recv()?;

        let mut vault = state.vault.write()?;
        *vault = match Vault::new_from_file(file.as_str(), password.as_bytes()) {
            Ok(vault) => Some(vault),
            Err(error) => return Err(GuiError::DatabaseError(error.to_string())),
        };

        State::append_vault_path_to_prev_vaults(state.clone(), file)?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    fn update_prev_vaults_max_length_internal(
        state: Arc<State>,
        new_max: usize,
    ) -> Result<(), GuiError> {
        let mut prev_vaults = state.prev_vaults.write()?;

        let max_len = &mut *state.prev_vaults_max_length.write()?;
        *max_len = new_max;
        if prev_vaults.len() > new_max {
            prev_vaults.resize(new_max, String::new());
        }

        Ok(())
    }

    pub async fn update_prev_vaults_max_length(state: Arc<State>, new_max: usize) {
        match State::update_prev_vaults_max_length_internal(state.clone(), new_max) {
            Ok(()) => (),
            Err(error) => GuiError::display_error_or_print(state, error),
        }
    }

    pub fn append_vault_path_to_prev_vaults(state: Arc<State>, file: String) -> Result<(), GuiError> {
        let mut prev_vaults = state.prev_vaults.write()?;

        let mut remove_list = VecDeque::new();
        for (index, val) in prev_vaults.iter().enumerate() {
            if val.eq(&file) {
                remove_list.push_front(index);
            }
        }

        for index in remove_list {
            let _ = prev_vaults.remove(index);
        }

        prev_vaults.push_front(file);

        let max_len = *state.prev_vaults_max_length.write()?;
        if prev_vaults.len() > max_len {
            prev_vaults.resize(max_len, String::new());
        }

        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn open_vault_from_file_dialog(state: Arc<State>) -> Result<(), GuiError> {
        let file = match Gui::open_file_dialog(state.clone()) {
            Some(file) => file.display().to_string(),
            None => return Err(GuiError::NoFile),
        };

        State::open_vault_from_file(state.clone(), file).await?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn save_vault_to_file(
        state: Arc<State>,
        path: &str,
        password: &[u8],
    ) -> Result<(), GuiError> {
        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.name_buffer = get_file_name(Path::new(path).to_path_buf());

        vault.serialize_to_file(&path, password)?;

        State::append_vault_path_to_prev_vaults(state.clone(), path.to_string())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn insert(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;

        let password = receiver.recv()?;
        let receiver =
            Self::add_password_prompt(state.clone(), format!("Enter entry for {}", name))?;

        let data = receiver.recv()?;

        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.insert(&name, data.as_bytes(), password.as_bytes())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn insert_from_csv(state: Arc<State>) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let file = match Gui::open_file_dialog(state.clone()) {
            Some(file) => file,
            None => return Err(GuiError::NoFile),
        };

        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.insert_from_csv(file.display().to_string().as_str(), password.as_bytes())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn export_to_csv(state: Arc<State>) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let file = match Gui::save_file_dialog(state.clone()) {
            Some(file) => file,
            None => return Err(GuiError::NoFile),
        };

        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.export_to_csv(file.display().to_string().as_str(), password.as_bytes())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn rename(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = State::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let receiver = State::add_prompt(state.clone(), format!("Enter new name"))?;
        let new_name = receiver.recv()?;

        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.rename(name.as_str(), new_name.as_str(), password.as_bytes())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn replace(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = State::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let receiver = State::add_password_prompt(state.clone(), format!("Enter new password"))?;
        let new_data = receiver.recv()?;

        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.replace(name.as_str(), new_data.as_bytes(), password.as_bytes())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn remove(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let mut vault = state.vault.write()?;
        let vault = match &mut *vault {
            Some(vault) => vault,
            None => return Err(GuiError::NoVault),
        };

        vault.remove(&name, password.as_bytes())?;
        state.egui_ctx.request_repaint();
        Ok(())
    }

    pub async fn get(state: Arc<State>, name: String) -> Result<(), GuiError> {
        let receiver = Self::add_password_prompt(state.clone(), format!("Enter master password"))?;
        let password = receiver.recv()?;

        let vault = state.vault.read()?;
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

        let mut string = state.clipboard_string.write()?;
        *string = Some(Zeroizing::new(result.to_string()));

        Ok(())
    }

    pub fn add_prompt(
        state: Arc<State>,
        prompt: String,
    ) -> Result<Receiver<Zeroizing<String>>, GuiError> {
        let (sender, receiver) = channel();

        let mut vec = state.prompts.write()?;
        vec.push(Prompt::new(
            prompt,
            Zeroizing::new(String::new()),
            sender,
            false,
        ));

        state.egui_ctx.request_repaint();
        Ok(receiver)
    }

    pub fn add_password_prompt(
        state: Arc<State>,
        prompt: String,
    ) -> Result<Receiver<Zeroizing<String>>, GuiError> {
        let (sender, receiver) = channel();

        let mut vec = state.prompts.write()?;
        vec.push(Prompt::new(
            prompt,
            Zeroizing::new(String::new()),
            sender,
            true,
        ));

        state.egui_ctx.request_repaint();
        Ok(receiver)
    }

    pub fn add_confirmation_password_prompt(
        state: Arc<State>,
        prompt: String,
        confirm_prompt: String,
    ) -> Result<Zeroizing<String>, GuiError> {
        let p1 = State::add_password_prompt(state.clone(), prompt)?.recv()?;
        let p2 = State::add_password_prompt(state, confirm_prompt)?.recv()?;

        if p1.eq(&p2) {
            return Ok(p1);
        } else {
            return Err(GuiError::PasswordNotSame);
        }
    }

    #[allow(unused)]
    pub fn add_message(state: Arc<State>, message: Message) -> Result<(), GuiError> {
        let mut messages = state.messages.write()?;
        messages.push(message);

        Ok(())
    }

    pub fn add_error(state: Arc<State>, error: String) -> Result<(), GuiError> {
        let mut messages = state.messages.write()?;

        let msg = Message::new_default_duration(Some(String::from("Error")), error, false);
        messages.push(msg);

        Ok(())
    }

    pub fn contains_vault(state: Arc<State>) -> Result<bool, GuiError> {
        if let Some(_vault) = &*state.vault.read()? {
            return Ok(true);
        }
        Ok(false)
    }

    pub fn get_prev_file(state: Arc<State>) -> Result<String, GuiError> {
        if let Some(vault) = &*state.vault.read()? {
            return Ok(vault.path.clone());
        }
        Err(GuiError::NoVault)
    }
}
