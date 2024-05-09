use std::path::Path;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
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

    prev_list_changed: bool,
    prev_list: Vec<String>,
    prev_pattern: String,
}

impl Vault {
    pub fn new(name: &str, password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        Ok(Self {
            db,
            changed: true,
            name_buffer: String::from(name),
            insert_buffer: String::new(),
            prev_list_changed: true,
            prev_list: Vec::new(),
            prev_pattern: String::new(),
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
            prev_list_changed: true,
            prev_list: Vec::new(),
            prev_pattern: String::new(),
        })
    }

    pub fn insert(
        &mut self,
        name: &String,
        data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        self.changed = true;
        self.prev_list_changed = true;
        self.db.insert(name, data, password)
    }

    pub fn insert_from_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        self.changed = true;
        self.prev_list_changed = true;
        self.db.insert_from_csv(file, password)
    }

    pub fn export_to_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        self.db.export_to_csv(file, password)
    }

    pub fn remove(&mut self, name: &String, password: &[u8]) -> Result<(), DatabaseError> {
        self.changed = true;
        self.prev_list_changed = true;
        self.db.remove(name, password)
    }

    pub fn get(&self, name: &String, password: &[u8]) -> Result<AesResult, DatabaseError> {
        self.db.get(name, password)
    }

    pub fn list(&self) -> Result<Vec<String>, DatabaseError> {
        self.db.list()
    }

    pub fn list_fuzzy_match(&mut self, pattern: &str) -> Result<Vec<String>, DatabaseError> {
        if !self.prev_list_changed && self.prev_pattern == pattern {
            return Ok(self.prev_list.clone());
        } else {
            let list = self.list()?;
            self.prev_list = list;
            self.prev_pattern = String::from(pattern);
            self.prev_list_changed = false;
        }

        let matcher = SkimMatcherV2::default();

        self.prev_list.sort_by(|a, b| {
            let a_score = match matcher.fuzzy_match(a.as_str(), pattern) {
                Some(score) => score,
                None => 0,
            };

            let b_score = match matcher.fuzzy_match(b.as_str(), pattern) {
                Some(score) => score,
                None => 0,
            };

            b_score.cmp(&a_score)
            // a_score.cmp(&b_score)
        });

        Ok(self.prev_list.clone())
    }

    pub fn serialize_to_file(&self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        let ciphertext = self.db.serialize_encrypted(password)?;
        std::fs::write(file, ciphertext.as_ref())?;

        Ok(())
    }
}
