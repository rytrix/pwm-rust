use crate::db_base::{Database, DatabaseError};
use pwm_lib::{
    aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult},
    hash::{
        argon2_wrapper::{argon2_hash_password, argon2_hash_password_with_salt},
        compare_hash,
        pbkdf2_wrapper::pbkdf2_hash_password_with_salt,
        randomize_slice, HashResult,
    },
};

use std::sync::Mutex;

pub struct DatabaseEncryptedAsync {
    db: Mutex<Database<AesResult>>,
    pw_hash: HashResult,
}

impl DatabaseEncryptedAsync {
    pub async fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let mut salt = [0; 32];
        randomize_slice(&mut salt);
        let hash = Self::hash_password(password, &salt)?;

        let db = Self {
            db: Mutex::new(Database::new()),
            pw_hash: hash,
        };

        return Ok(db);
    }

    pub async fn new_deserialize(serialized: &[u8]) -> Result<Self, DatabaseError> {
        let hash = match HashResult::new_with_salt_and_hash(
            &serialized[serialized.len() - 32..],
            &serialized[serialized.len() - 64..serialized.len() - 32],
        ) {
            Ok(hash) => hash,
            Err(_error) => return Err(DatabaseError::FailedDeserialize),
        };

        let db: Database<AesResult> =
            match bincode::deserialize(&serialized[..serialized.len() - 64]) {
                Ok(db) => db,
                Err(_error) => return Err(DatabaseError::FailedDeserialize),
            };

        Ok(Self {
            db: Mutex::new(db),
            pw_hash: hash,
        })
    }

    pub async fn insert(
        &mut self,
        name: &str,
        data: &[u8],
        password: &[u8],
    ) -> Result<(), DatabaseError> {
        let valid_password = self.hash_password_and_compare(password);

        let hash = match argon2_hash_password(password) {
            Ok(hash_result) => hash_result,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        if !valid_password.await {
            return Err(DatabaseError::InvalidPassword);
        }

        let data = match aes_gcm_encrypt(&hash, data) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        let db = match self.db.get_mut() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };
        db.insert(name, data)?;

        Ok(())
    }

    pub async fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError> {
        let valid_password = self.hash_password_and_compare(password);
        if !valid_password.await {
            return Err(DatabaseError::InvalidPassword);
        }

        let db = match self.db.get_mut() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };

        db.remove(name)?;

        Ok(())
    }

    pub async fn get(&self, name: &str, password: &[u8]) -> Result<AesResult, DatabaseError> {
        let valid_password = self.hash_password_and_compare(password);
        if !valid_password.await {
            return Err(DatabaseError::InvalidPassword);
        }

        let db = match self.db.try_lock() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };

        let ciphertext = db.get(name)?;

        let hash = match argon2_hash_password_with_salt(password, ciphertext.get_salt_slice()) {
            Ok(hash_result) => hash_result,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        let result = match aes_gcm_decrypt(hash.get_hash(), ciphertext) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        Ok(result)
    }

    fn hash_password(password: &[u8], salt: &[u8]) -> Result<HashResult, DatabaseError> {
        let hash = match pbkdf2_hash_password_with_salt(password, salt) {
            Ok(hash) => hash,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        Ok(hash)
    }

    // Returns true if the hash matches
    async fn hash_password_and_compare(&self, password: &[u8]) -> bool {
        let result = match Self::hash_password(password, self.pw_hash.get_salt()) {
            Ok(hash) => hash,
            Err(_error) => return false,
        };

        compare_hash(result.get_hash(), self.pw_hash.get_hash())
    }

    pub async fn serialize(&mut self) -> Result<Vec<u8>, DatabaseError> {
        let db = match self.db.try_lock() {
            Ok(db) => db,
            Err(_) => return Err(DatabaseError::LockError),
        };

        let mut data = match bincode::serialize(db.as_ref()) {
            Ok(data) => data,
            Err(_err) => return Err(DatabaseError::FailedDeserialize),
        };

        data.extend_from_slice(self.pw_hash.get_hash());
        data.extend_from_slice(self.pw_hash.get_salt());

        Ok(data)
    }
}

#[cfg(test)]
mod test {
    use super::DatabaseEncryptedAsync;

    #[test]
    fn test_generic() {
        tokio_test::block_on(test_generic_async())
    }

    async fn test_generic_async() {
        let mut db = DatabaseEncryptedAsync::new(b"test").await.unwrap();
        db.insert("ryan", b"password", b"test").await.unwrap();
        db.insert("ryan2", b"password", b"test").await.unwrap();

        let pass = db.get("ryan", b"test").await.unwrap();
        assert_eq!(b"password", pass.as_slice());
        db.remove("ryan2", b"test").await.unwrap();
        db.remove("ryan", b"test").await.unwrap();
    }

    #[test]
    fn test_serialize_deserialize() {
        tokio_test::block_on(test_serialize_deserialize_async())
    }

    async fn test_serialize_deserialize_async() {
        let mut db = DatabaseEncryptedAsync::new(b"test").await.unwrap();
        db.insert("ryan", b"password", b"test").await.unwrap();
        db.insert("ryan2", b"password", b"test").await.unwrap();

        let serialized = db.serialize().await.unwrap();
        let db = DatabaseEncryptedAsync::new_deserialize(serialized.as_slice())
            .await
            .unwrap();

        let pass = db.get("ryan", b"test").await.unwrap();
        assert_eq!(b"password", pass.as_slice())
    }
}
