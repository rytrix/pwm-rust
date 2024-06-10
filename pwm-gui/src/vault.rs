use std::path::Path;

use log::info;
use pwm_db::{
    db_base::error::DatabaseError,
    db_encrypted::{db_interface::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::encryption::EncryptionResult;

use crate::gui::get_file_name;

pub struct Vault {
    db: DatabaseEncrypted,
    pub changed: bool,
    pub path: String,
    pub name_buffer: String,
}

impl Vault {
    pub fn new(name: &str, password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        let path = std::env::current_exe()?;
        let path = path.display().to_string() + "/" + name;
        info!("New Vault with name: \"{}\" and path \"{}\"", name, path);
        Ok(Self {
            db,
            changed: true,
            path,
            name_buffer: String::from(name),
        })
    }

    pub fn new_from_file(file: &str, password: &[u8]) -> Result<Self, DatabaseError> {
        let contents = match std::fs::read(file) {
            Ok(contents) => match EncryptionResult::new(contents) {
                Ok(contents) => contents,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            },
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new_deserialize_encrypted(&contents, password)?;

        let path = Path::new(file);
        let name = get_file_name(path.to_path_buf());

        let path = file.to_string();
        info!("New Vault with name: \"{}\" and path \"{}\"", name, path);

        Ok(Self {
            db,
            changed: false,
            path,
            name_buffer: name,
        })
    }

    pub fn insert(
        &mut self,
        name: &str,
        data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        self.changed = true;
        self.db.insert(name, data, password)
    }

    pub fn insert_from_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        self.changed = true;
        self.db.insert_from_csv(file, password)
    }

    pub fn export_to_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        self.db.export_to_csv(file, password)
    }

    pub fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError> {
        self.changed = true;
        self.db.remove(name, password)
    }

    pub fn replace(
        &mut self,
        name: &str,
        new_data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        self.changed = true;
        self.db.replace(name, new_data, password)
    }

    pub fn rename(
        &mut self,
        name: &str,
        new_name: &str,
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        if name == new_name {
            return Ok(())
        }
        self.changed = true;
        self.db.rename(name, new_name, password)
    }

    pub fn get(&self, name: &str, password: &[u8]) -> Result<EncryptionResult, DatabaseError> {
        self.db.get(name, password)
    }

    #[allow(unused)]
    pub fn list(&self) -> Result<Vec<String>, DatabaseError> {
        self.db.list()
    }

    pub fn list_fuzzy_match(&mut self, pattern: &str) -> Result<&Vec<String>, DatabaseError> {
        self.db.list_fuzzy_match(pattern)
    }

    pub fn serialize_to_file(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        let ciphertext = self.db.serialize_encrypted(password)?;
        std::fs::write(file, ciphertext.as_ref())?;
        self.changed = false;
        self.path = file.to_string();
        self.name_buffer = get_file_name(Path::new(file).to_path_buf());
        info!("File saved to file, with name \"{}\" and path \"{}\"", self.name_buffer, self.path);

        Ok(())
    }
}
