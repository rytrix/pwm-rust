use pwm_db::{
    db_base::DatabaseError,
    db_encrypted::{keep_hash::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::aes_wrapper::AesResult;
struct Vault {
    db: DatabaseEncrypted,
    changed: bool,
}

impl Vault {
    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        Ok(Self { db, changed: true })
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

        Ok(Self { db, changed: false })
    }

    pub fn insert(&mut self, name: &String, data: &[u8]) -> Result<(), DatabaseError> {
        self.db.insert(name, data)
    }

    pub fn remove(&mut self, name: &String) -> Result<(), DatabaseError> {
        self.db.remove(name)
    }

    pub fn get(&self, name: &String) -> Result<AesResult, DatabaseError> {
        self.db.get(name)
    }

    pub fn serialize_to_file(&self, file: &str) -> Result<(), DatabaseError> {
        let ciphertext = self.db.serialize_encrypted()?;
        match std::fs::write(file, ciphertext.as_ref()) {
            Ok(()) => (),
            Err(error) => return Err(DatabaseError::OutputError(error.to_string())),
        };

        Ok(())
    }
}
