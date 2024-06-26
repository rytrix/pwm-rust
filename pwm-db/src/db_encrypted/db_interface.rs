use log::{trace, warn};
#[cfg(feature = "use-compression")]
use lz4_flex::compress_prepend_size;

use pwm_lib::{
    encryption::{
        default::{decrypt, encrypt},
        EncryptionResult,
    },
    zeroize::Zeroizing,
};

use crate::db_base::error::DatabaseError;

use super::DatabaseEncrypted;

pub trait DatabaseInterface {
    fn new_deserialize_encrypted(
        serialized: &EncryptionResult,
        password: &[u8],
    ) -> Result<DatabaseEncrypted, DatabaseError>;
    fn insert(&mut self, name: &str, data: &[u8], password: &[u8]) -> Result<(), DatabaseError>;
    fn insert_from_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn export_to_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn replace(
        &mut self,
        name: &str,
        new_data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError>;
    fn rename(&mut self, name: &str, new_name: &str, password: &[u8]) -> Result<(), DatabaseError>;
    fn get(&self, name: &str, password: &[u8]) -> Result<EncryptionResult, DatabaseError>;
    fn serialize_encrypted(&self, password: &[u8]) -> Result<EncryptionResult, DatabaseError>;
}

impl DatabaseInterface for DatabaseEncrypted {
    fn new_deserialize_encrypted(
        serialized: &EncryptionResult,
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
        let data = encrypt(data, &hash)?;

        self.db.insert(name, data)?;

        Ok(())
    }

    fn insert_from_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let mut failed_records = Vec::new();
        let mut rdr = csv::Reader::from_path(file)?;
        for record in rdr.records() {
            match record {
                Ok(record) => {
                    if let (Some(key), Some(data)) = (record.get(0), record.get(1)) {
                        let hash = Self::hash_password_argon2(password)?;
                        let data = encrypt(data.as_bytes(), &hash)?;

                        match self.db.insert(key, data) {
                            Ok(()) => (),
                            Err(error) => {
                                failed_records.push(String::from(key));
                                warn!("pwm-db: Failed to import: {}", error.to_string());
                            }
                        };
                    }
                    trace!(
                        "pwm-db: attempted to import record: {:?}, {:?}",
                        record.get(0),
                        record.get(1)
                    );
                }
                Err(_) => {}
            };
        }

        if failed_records.len() == 0 {
            Ok(())
        } else {
            let mut error_msg = String::new();
            for (index, record) in failed_records.iter().enumerate() {
                if index != failed_records.len() - 1 {
                    let record = record.clone();
                    error_msg.push_str((record + ", ").as_str());
                } else {
                    error_msg.push_str(record.as_str());
                }
            }
            Err(DatabaseError::ImportError(error_msg))
        }
    }

    fn export_to_csv(&mut self, file: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }
        let mut writer = csv::Writer::from_path(file)?;

        writer.write_record([b"Username", b"Password"])?;
        for name in self.db.list()? {
            let ciphertext = self.db.get(name.as_str())?;
            let hash = Self::hash_password_argon2_with_salt(password, ciphertext.get_salt_slice())?;

            let result = decrypt(ciphertext, &hash)?;

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

    fn replace(
        &mut self,
        name: &str,
        new_data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let hash = Self::hash_password_argon2(password)?;
        let data = encrypt(new_data, &hash)?;

        self.db.replace(name, data)?;

        Ok(())
    }

    fn rename(&mut self, name: &str, new_name: &str, password: &[u8]) -> Result<(), DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        self.db.rename(name, new_name)?;

        Ok(())
    }

    fn get(&self, name: &str, password: &[u8]) -> Result<EncryptionResult, DatabaseError> {
        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let ciphertext = self.db.get(name)?;
        let hash = Self::hash_password_argon2_with_salt(password, ciphertext.get_salt_slice())?;

        let result = decrypt(ciphertext, &hash)?;

        Ok(result)
    }

    fn serialize_encrypted(&self, password: &[u8]) -> Result<EncryptionResult, DatabaseError> {
        let data = self.serialize()?;

        if !self.hash_password_and_compare(password) {
            return Err(DatabaseError::InvalidPassword);
        }

        #[cfg(feature = "use-compression")]
        let data = Zeroizing::new(compress_prepend_size(data.as_slice()));

        let hash = Self::hash_password_argon2(password)?;
        let ciphertext = encrypt(data.as_slice(), &hash)?;

        Ok(ciphertext)
    }
}
