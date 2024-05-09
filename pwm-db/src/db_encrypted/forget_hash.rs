use pwm_lib::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult};

use crate::db_base::DatabaseError;

use super::DatabaseEncrypted;

pub trait DatabaseInterface {
    fn new_deserialize_encrypted(
        serialized: &AesResult,
        password: &[u8],
    ) -> Result<DatabaseEncrypted, DatabaseError>;
    fn insert(&mut self, name: &str, data: &[u8], password: &[u8]) -> Result<(), DatabaseError>;
    fn insert_from_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn export_to_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn get(&self, name: &str, password: &[u8]) -> Result<AesResult, DatabaseError>;
    fn serialize_encrypted(&self, password: &[u8]) -> Result<AesResult, DatabaseError>;
}

impl DatabaseInterface for DatabaseEncrypted {
    fn new_deserialize_encrypted(
        serialized: &AesResult,
        password: &[u8],
    ) -> Result<DatabaseEncrypted, DatabaseError> {
        let (db, _hash) = Self::new_deserialize_encrypted_internal(serialized, password)?;
        Ok(db)
    }

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

    fn insert_from_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let mut rdr = csv::Reader::from_path(file)?;
        for record in rdr.records() {
            match record {
                Ok(record) => {
                    if let (Some(key), Some(data)) = (record.get(0), record.get(1)) {
                        let hash = Self::hash_password_argon2(password)?;

                        let data = aes_gcm_encrypt(&hash, data.as_bytes())?;
                        self.db.insert(key, data)?;
                    }
                    // println!("record {:?} {:?}", record.get(0), record.get(1));
                }
                Err(_) => {}
            };
        }

        Ok(())
    }

    fn export_to_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }
        let mut writer = csv::Writer::from_path(file)?;

        for name in self.db.list()? {
            let ciphertext = self.db.get(name.as_str())?;
            let hash = Self::hash_password_argon2_with_salt(password, ciphertext.get_salt_slice())?;

            let result = aes_gcm_decrypt(hash.get_hash(), ciphertext)?;

            writer.write_record([name.as_bytes(), result.as_slice()])?;
        }

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

        let result = aes_gcm_decrypt(hash.get_hash(), ciphertext)?;

        Ok(result)
    }

    fn serialize_encrypted(&self, password: &[u8]) -> Result<AesResult, DatabaseError> {
        let data = self.serialize()?;

        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }
        let hash = Self::hash_password_argon2(password)?;

        let ciphertext = aes_gcm_encrypt(&hash, data.as_slice())?;

        Ok(ciphertext)
    }
}
