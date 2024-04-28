use pwm_lib::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult};

use crate::db_base::DatabaseError;

use super::DatabaseEncrypted;

pub trait DatabaseInterface {
    fn insert(&mut self, name: &str, data: &[u8]) -> Result<(), DatabaseError>;
    fn remove(&mut self, name: &str) -> Result<(), DatabaseError>;
    fn get(&self, name: &str) -> Result<AesResult, DatabaseError>;
    fn serialize_encrypted(&self) -> Result<AesResult, DatabaseError>;
}

impl DatabaseInterface for DatabaseEncrypted {
    fn insert(&mut self, name: &str, data: &[u8]) -> Result<(), DatabaseError> {
        let data = match aes_gcm_encrypt(&self.pw_hash, data) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        self.db.insert(name, data)?;

        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        self.db.remove(name)?;

        Ok(())
    }

    fn get(&self, name: &str) -> Result<AesResult, DatabaseError> {
        let ciphertext = self.db.get(name)?;

        let result = match aes_gcm_decrypt(self.pw_hash.get_hash(), ciphertext) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(result)
    }

    fn serialize_encrypted(&self) -> Result<AesResult, DatabaseError> {
        let data = self.serialize()?;

        let ciphertext = match aes_gcm_encrypt(&self.pw_hash, data.as_slice()) {
            Ok(ciphertext) => ciphertext,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(ciphertext)
    }
}
