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

pub struct DatabaseEncrypted {
    db: Database<AesResult>,
    pw_hash: HashResult,
}

impl DatabaseEncrypted {
    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let mut salt = [0; 32];
        randomize_slice(&mut salt);
        let hash = Self::hash_password(password, &salt)?;

        let db = Self {
            db: Database::new(),
            pw_hash: hash,
        };

        return Ok(db);
    }

    pub fn new_deserialize(serialized: &[u8]) -> Result<Self, DatabaseError> {
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
            db: db,
            pw_hash: hash,
        })
    }

    pub fn insert(
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

        if !valid_password {
            return Err(DatabaseError::InvalidPassword);
        }

        let data = match aes_gcm_encrypt(&hash, data) {
            Ok(encrypted) => encrypted,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        self.db.insert(name, data)?;

        Ok(())
    }

    pub fn remove(&mut self, name: &str, password: &[u8]) -> Result<(), DatabaseError> {
        let valid_password = self.hash_password_and_compare(password);
        if !valid_password {
            return Err(DatabaseError::InvalidPassword);
        }

        self.db.remove(name)?;

        Ok(())
    }

    pub fn get(&self, name: &str, password: &[u8]) -> Result<AesResult, DatabaseError> {
        let valid_password = self.hash_password_and_compare(password);
        if !valid_password {
            return Err(DatabaseError::InvalidPassword);
        }

        let ciphertext = self.db.get(name)?;

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

    pub fn list(&self) -> Result<Vec<String>, DatabaseError> {
        self.db.list()
    }

    fn hash_password(password: &[u8], salt: &[u8]) -> Result<HashResult, DatabaseError> {
        let hash = match pbkdf2_hash_password_with_salt(password, salt) {
            Ok(hash) => hash,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
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

    pub fn serialize(&self) -> Result<Vec<u8>, DatabaseError> {
        let mut data = match bincode::serialize(self.db.as_ref()) {
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
    use super::DatabaseEncrypted;

    #[test]
    fn test_generic() {
        let mut db = DatabaseEncrypted::new(b"test").unwrap();
        db.insert("ryan", b"password", b"test").unwrap();
        db.insert("ryan2", b"password", b"test").unwrap();

        let pass = db.get("ryan", b"test").unwrap();
        assert_eq!(b"password", pass.as_slice());
        db.remove("ryan2", b"test").unwrap();
        db.remove("ryan", b"test").unwrap();
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut db = DatabaseEncrypted::new(b"test").unwrap();
        db.insert("ryan", b"password", b"test").unwrap();
        db.insert("ryan2", b"password", b"test").unwrap();

        let serialized = db.serialize().unwrap();
        let db = DatabaseEncrypted::new_deserialize(serialized.as_slice()).unwrap();

        let pass = db.get("ryan", b"test").unwrap();
        assert_eq!(b"password", pass.as_slice())
    }
}
