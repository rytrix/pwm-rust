use pwm_lib::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult};

use crate::db_base::DatabaseError;

use super::DatabaseEncrypted;

pub trait DatabaseInterface {
    fn insert(&mut self, name: &str, data: &[u8], password: &[u8]) -> Result<(), DatabaseError>;
    fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn get(&self, name: &str, password: &[u8]) -> Result<AesResult, DatabaseError>;
    fn serialize_encrypted(&self, password: &[u8]) -> Result<AesResult, DatabaseError>;
}

impl DatabaseInterface for DatabaseEncrypted {
    fn insert(&mut self, name: &str, data: &[u8], password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let hash = Self::hash_password_argon2(password)?;

        let data = match aes_gcm_encrypt(&hash, data) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        self.db.insert(name, data)?;

        Ok(())
    }

    fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        self.db.remove(name)?;

        Ok(())
    }

    fn get(&self, name: &str, password: &[u8]) -> Result<AesResult, DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let ciphertext = self.db.get(name)?;

        let hash = Self::hash_password_argon2_with_salt(password, ciphertext.get_salt_slice())?;

        let result = match aes_gcm_decrypt(hash.get_hash(), ciphertext) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(result)
    }

    fn serialize_encrypted(&self, password: &[u8]) -> Result<AesResult, DatabaseError> {
        let data = self.serialize()?;

        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }
        let hash = Self::hash_password_argon2(password)?;

        let ciphertext = match aes_gcm_encrypt(&hash, data.as_slice()) {
            Ok(ciphertext) => ciphertext,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(ciphertext)
    }
}
