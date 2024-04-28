use pwm_lib::{aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult}, hash::HashResult};

use crate::db_base::DatabaseError;

use super::DatabaseEncrypted;

pub trait DatabaseInterface {
    fn new_deserialize_encrypted(
        serialized: &AesResult,
        password: &[u8],
    ) -> Result<(DatabaseEncrypted, HashResult), DatabaseError>;
    fn insert(&mut self, name: &str, data: &[u8], hash: &HashResult) -> Result<(), DatabaseError>;
    fn remove(&mut self, name: &str, hash: &HashResult) -> Result<(), DatabaseError>;
    fn get(&self, name: &str, hash: &HashResult) -> Result<AesResult, DatabaseError>;
    fn serialize_encrypted(&self, hash: &HashResult) -> Result<AesResult, DatabaseError>;
}

impl DatabaseInterface for DatabaseEncrypted {
    fn new_deserialize_encrypted(
        serialized: &AesResult,
        password: &[u8],
    ) -> Result<(DatabaseEncrypted, HashResult), DatabaseError>
    {
        Self::new_deserialize_encrypted_internal(serialized, password)
    }
    fn insert(&mut self, name: &str, data: &[u8], hash: &HashResult) -> Result<(), DatabaseError> {
        let data = match aes_gcm_encrypt(hash, data) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        self.db.insert(name, data)?;

        Ok(())
    }

    fn remove(&mut self, name: &str, _hash: &HashResult) -> Result<(), DatabaseError> {
        self.db.remove(name)?;

        Ok(())
    }

    fn get(&self, name: &str, hash: &HashResult) -> Result<AesResult, DatabaseError> {
        let ciphertext = self.db.get(name)?;

        let result = match aes_gcm_decrypt(hash.get_hash(), ciphertext) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(result)
    }

    fn serialize_encrypted(&self, hash: &HashResult) -> Result<AesResult, DatabaseError> {
        let data = self.serialize()?;

        let ciphertext = match aes_gcm_encrypt(hash, data.as_slice()) {
            Ok(ciphertext) => ciphertext,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(ciphertext)
    }
}
