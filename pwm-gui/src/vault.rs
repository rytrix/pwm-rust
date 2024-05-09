use std::path::Path;

use pwm_db::{
    db_base::DatabaseError,
    db_encrypted::{forget_hash::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::aes_wrapper::AesResult;

use crate::gui::get_file_name;

pub struct Vault {
    db: DatabaseEncrypted,
    changed: bool,
    pub name_buffer: String,
    pub insert_buffer: String,
}

impl Vault {
    pub fn new(name: &str, password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        Ok(Self {
            db,
            changed: true,
            name_buffer: String::from(name),
            insert_buffer: String::new(),
        })
    }

    pub fn new_from_file(file: &str, password: &[u8]) -> Result<Self, DatabaseError> {
        let contents = match std::fs::read(file) {
            Ok(contents) => match AesResult::new(contents) {
                Ok(contents) => contents,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            },
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new_deserialize_encrypted(&contents, password)?;

        let path = Path::new(file);
        let name = get_file_name(path.to_path_buf());

        Ok(Self {
            db,
            changed: false,
            name_buffer: name,
            insert_buffer: String::new(),
        })
    }

    pub fn insert(
        &mut self,
        name: &String,
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

    pub fn remove(&mut self, name: &String, password: &[u8]) -> Result<(), DatabaseError> {
        self.changed = true;
        self.db.remove(name, password)
    }

    pub fn get(&self, name: &String, password: &[u8]) -> Result<AesResult, DatabaseError> {
        self.db.get(name, password)
    }

    pub fn list(&self) -> Result<Vec<String>, DatabaseError> {
        self.db.list()
    }

    pub fn serialize_to_file(&self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        let ciphertext = self.db.serialize_encrypted(password)?;
        match std::fs::write(file, ciphertext.as_ref()) {
            Ok(()) => (),
            Err(error) => return Err(DatabaseError::OutputError(error.to_string())),
        };

        Ok(())
    }
}
