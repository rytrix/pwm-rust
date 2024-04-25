use crate::db_base::{Database, DatabaseError};
use pwm_lib::{
    aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult},
    hash::{
        argon2_wrapper::argon2_hash_password, compare_hash,
        pbkdf2_wrapper::pbkdf2_hash_password_with_salt, randomize_slice, HashResult,
    },
};

use std::sync::Mutex;

pub struct DatabaseEncryptedAsync {
    db: Mutex<Database<AesResult>>,
    pw_hash: HashResult,
}

impl DatabaseEncryptedAsync {
    fn hash_password(password: &[u8], salt: &[u8]) -> Result<HashResult, DatabaseError> {
        let hash = match pbkdf2_hash_password_with_salt(password, salt) {
            Ok(hash) => hash,
            Err(_error) => return Err(DatabaseError::FailedHash),
        };

        Ok(hash)
    }

    // Returns true if the hash matches
    fn hash_password_and_compare(&self, password: &[u8]) -> bool {
        let result = match Self::hash_password(password, self.pw_hash.get_salt()) {
            Ok(hash) => hash,
            Err(_error) => return false,
        };

        compare_hash(result.get_hash(), self.pw_hash.get_hash())
    }

    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let mut salt = [0; 32];
        randomize_slice(&mut salt);
        let hash = Self::hash_password(password, &salt)?;

        let db = Self {
            db: Mutex::new(Database::new()),
            pw_hash: hash,
        };

        return Ok(db);
    }

    pub async fn insert(
        &mut self,
        name: &str,
        data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let hash = match argon2_hash_password(password) {
            Ok(hash_result) => hash_result,
            Err(_error) => return Err(DatabaseError::FailedHash),
        };

        let data = match aes_gcm_encrypt(&hash, data) {
            Ok(encrypted) => encrypted,
            Err(_error) => return Err(DatabaseError::FailedAes),
        };

        let db = match self.db.get_mut() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };
        db.insert(name, data)?;

        Ok(())
    }

    pub async fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let db = match self.db.get_mut() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };

        db.remove(name)?;

        Ok(())
    }

    pub async fn get(&mut self, name: &str, password: &[u8]) -> Result<AesResult, DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let db = match self.db.get_mut() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };

        let ciphertext = db.get(name)?;

        let hash = match argon2_hash_password(password) {
            Ok(hash_result) => hash_result,
            Err(_error) => return Err(DatabaseError::FailedHash),
        };

        let result = match aes_gcm_decrypt(hash.get_hash(), ciphertext) {
            Ok(encrypted) => encrypted,
            Err(_error) => return Err(DatabaseError::FailedAes),
        };

        Ok(result)
    }
}
