// TODO this crate is bad and it needs to be DELETED, convince me otherwise, 
// you NEED to rehash or you will keep the same salt for everything

use pwm_lib::{
    aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult},
    hash::HashResult,
};

use crate::db_base::DatabaseError;

use super::DatabaseEncrypted;

pub trait DatabaseInterface {
    fn new_deserialize_encrypted(
        serialized: &AesResult,
        password: &[u8],
    ) -> Result<(DatabaseEncrypted, HashResult), DatabaseError>;
    fn insert(&mut self, name: &str, data: &[u8], hash: &HashResult) -> Result<(), DatabaseError>;
    fn insert_from_csv(&mut self, file: &str, hash: &HashResult) -> Result<(), DatabaseError>;
    fn remove(&mut self, name: &str, hash: &HashResult) -> Result<(), DatabaseError>;
    fn get(&self, name: &str, hash: &HashResult) -> Result<AesResult, DatabaseError>;
    fn serialize_encrypted(&self, hash: &HashResult) -> Result<AesResult, DatabaseError>;
}

impl DatabaseInterface for DatabaseEncrypted {
    fn new_deserialize_encrypted(
        serialized: &AesResult,
        password: &[u8],
    ) -> Result<(DatabaseEncrypted, HashResult), DatabaseError> {
        Self::new_deserialize_encrypted_internal(serialized, password)
    }

    fn insert(&mut self, name: &str, data: &[u8], hash: &HashResult) -> Result<(), DatabaseError> {
        let data = aes_gcm_encrypt(hash, data)?;
        self.db.insert(name, data)?;

        Ok(())
    }

    fn insert_from_csv(&mut self, file: &str, hash: &HashResult) -> Result<(), DatabaseError> {
        let mut rdr = csv::Reader::from_path(file)?;
        for record in rdr.records() {
            match record {
                Ok(record) => {
                    if let (Some(key), Some(data)) = (record.get(0), record.get(1)) {
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

    fn remove(&mut self, name: &str, _hash: &HashResult) -> Result<(), DatabaseError> {
        self.db.remove(name)?;

        Ok(())
    }

    fn get(&self, name: &str, hash: &HashResult) -> Result<AesResult, DatabaseError> {
        let ciphertext = self.db.get(name)?;

        let result = aes_gcm_decrypt(hash.get_hash(), ciphertext)?;
        Ok(result)
    }

    fn serialize_encrypted(&self, hash: &HashResult) -> Result<AesResult, DatabaseError> {
        let data = self.serialize()?;

        let ciphertext = aes_gcm_encrypt(hash, data.as_slice())?;

        Ok(ciphertext)
    }
}
