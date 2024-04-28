use pwm_db::{
    db_base::DatabaseError,
    db_encrypted::{keep_hash::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::{
    aes_wrapper::AesResult,
    hash::{argon2_wrapper::argon2_hash_password, HashResult},
};

struct Vault {
    db: DatabaseEncrypted,
    hash: HashResult,
    changed: bool,
}

impl Vault {
    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        let hash = match argon2_hash_password(password) {
            Ok(value) => value,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };
        Ok(Self {
            db,
            hash,
            changed: true,
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

        let (db, hash) = DatabaseEncrypted::new_deserialize_encrypted(&contents, password)?;

        Ok(Self {
            db,
            hash,
            changed: false,
        })
    }

    pub fn insert(&mut self, name: &String, data: &[u8]) -> Result<(), DatabaseError> {
        self.db.insert(name, data, &self.hash)
    }

    pub fn remove(&mut self, name: &String) -> Result<(), DatabaseError> {
        self.db.remove(name, &self.hash)
    }

    pub fn get(&self, name: &String) -> Result<AesResult, DatabaseError> {
        self.db.get(name, &self.hash)
    }

    pub fn serialize_to_file(&self, file: &str) -> Result<(), DatabaseError> {
        let ciphertext = self.db.serialize_encrypted(&self.hash)?;
        match std::fs::write(file, ciphertext.as_ref()) {
            Ok(()) => (),
            Err(error) => return Err(DatabaseError::OutputError(error.to_string())),
        };

        Ok(())
    }
}
