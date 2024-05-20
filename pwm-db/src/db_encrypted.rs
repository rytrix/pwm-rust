use crate::db_base::{error::DatabaseError, Database};
use pwm_lib::{
    encryption::{default::decrypt, EncryptionResult},
    hash::{
        argon2_wrapper::{argon2_hash_password, argon2_hash_password_with_salt},
        compare_hash,
        pbkdf2_wrapper::{pbkdf2_hash_password, pbkdf2_hash_password_with_salt},
        HashResult,
    },
    zeroize::Zeroizing,
};

pub struct DatabaseEncrypted {
    db: Database<EncryptionResult>,
    confirmation_hash: HashResult,
}

impl DatabaseEncrypted {
    // Common
    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let hash = Self::hash_password_pbkdf2(password)?;

        let db = Self {
            db: Database::new(),
            confirmation_hash: hash,
        };

        Ok(db)
    }

    fn new_deserialize(serialized: &[u8], password: &[u8]) -> Result<Self, DatabaseError> {
        let hash = match HashResult::new_with_salt_and_hash(
            &serialized[serialized.len() - 32..],
            &serialized[serialized.len() - 64..serialized.len() - 32],
        ) {
            Ok(hash) => hash,
            Err(_error) => return Err(DatabaseError::FailedDeserialize),
        };

        if !Self::hash_password_and_compare_internal(&hash, password) {
            return Err(DatabaseError::InvalidPassword);
        }

        let db: Database<EncryptionResult> =
            match bincode::deserialize(&serialized[..serialized.len() - 64]) {
                Ok(db) => db,
                Err(_error) => return Err(DatabaseError::FailedDeserialize),
            };

        Ok(Self {
            db,
            confirmation_hash: hash,
        })
    }

    fn new_deserialize_encrypted_internal(
        serialized: &EncryptionResult,
        password: &[u8],
    ) -> Result<(Self, HashResult), DatabaseError> {
        let hash = Self::hash_password_argon2_with_salt(password, serialized.get_salt_slice())?;

        let plaintext = match decrypt(serialized, &hash) {
            Ok(plaintext) => plaintext,
            Err(error) => return Err(DatabaseError::FailedAes(error.to_string())),
        };

        let result = Self::new_deserialize(plaintext.as_slice(), password)?;

        Ok((result, hash))
    }

    fn serialize(&self) -> Result<Zeroizing<Vec<u8>>, DatabaseError> {
        let mut data = match bincode::serialize(self.db.as_ref()) {
            Ok(data) => Zeroizing::new(data),
            Err(_err) => return Err(DatabaseError::FailedDeserialize),
        };

        data.extend_from_slice(self.confirmation_hash.get_hash());
        data.extend_from_slice(self.confirmation_hash.get_salt());

        Ok(data)
    }

    pub fn list(&self) -> Result<Vec<String>, DatabaseError> {
        self.db.list()
    }

    pub fn list_fuzzy_match(&mut self, pattern: &str) -> Result<&Vec<String>, DatabaseError> {
        self.db.list_fuzzy_match(pattern)
    }

    // Common end

    // Utility
    fn hash_password_pbkdf2_with_salt(
        password: &[u8],
        salt: &[u8],
    ) -> Result<HashResult, DatabaseError> {
        let hash = match pbkdf2_hash_password_with_salt(password, salt) {
            Ok(hash) => hash,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        Ok(hash)
    }

    fn hash_password_pbkdf2(password: &[u8]) -> Result<HashResult, DatabaseError> {
        let hash = match pbkdf2_hash_password(password) {
            Ok(hash) => hash,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        Ok(hash)
    }

    fn hash_password_argon2_with_salt(
        password: &[u8],
        salt: &[u8],
    ) -> Result<HashResult, DatabaseError> {
        let hash = match argon2_hash_password_with_salt(password, salt) {
            Ok(value) => value,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        Ok(hash)
    }

    fn hash_password_argon2(password: &[u8]) -> Result<HashResult, DatabaseError> {
        let hash = match argon2_hash_password(password) {
            Ok(value) => value,
            Err(error) => return Err(DatabaseError::FailedHash(error.to_string())),
        };

        Ok(hash)
    }

    fn hash_password_and_compare_internal(hash: &HashResult, password: &[u8]) -> bool {
        let result = match Self::hash_password_pbkdf2_with_salt(password, hash.get_salt()) {
            Ok(hash) => hash,
            Err(_error) => return false,
        };

        compare_hash(result.get_hash(), hash.get_hash())
    }

    // Returns true if the hash matches
    fn hash_password_and_compare(&self, password: &[u8]) -> bool {
        Self::hash_password_and_compare_internal(&self.confirmation_hash, password)
    }
    // End Utility
}

#[cfg(feature = "keep-hash")]
#[deprecated(
    since = "0.0.1",
    note = "Please don't ever use this it uses the same salt for EVERYTHING!"
)]
pub mod keep_hash;

pub mod forget_hash;

#[cfg(test)]
mod test_forget {
    use crate::db_encrypted::forget_hash::DatabaseInterface;

    use super::DatabaseEncrypted;

    #[test]
    fn test_generic() {
        let mut db = DatabaseEncrypted::new(b"test").unwrap();
        db.insert("user", b"password", b"test").unwrap();
        db.insert("user2", b"password", b"test").unwrap();
        let list = db.list().unwrap();
        assert_eq!(list.contains(&"user".to_string()), true);
        assert_eq!(list.contains(&"user2".to_string()), true);

        let pass = db.get("user", b"test").unwrap();
        assert_eq!(b"password", pass.as_slice());
        db.remove("user2", b"test").unwrap();
        db.remove("user", b"test").unwrap();
        let list = db.list().unwrap();
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_generic2() {
        let pw = b"test";
        let mut db = DatabaseEncrypted::new(pw).unwrap();
        db.insert("user", b"password", pw).unwrap();
        db.insert("user2", b"password", pw).unwrap();
        db.rename("user2", "user1", pw).unwrap();
        let list = db.list().unwrap();
        assert_eq!(list.contains(&"user".to_string()), true);
        assert_eq!(list.contains(&"user1".to_string()), true);

        db.replace("user1", b"password_new", pw).unwrap();
        let result = db.get("user1", pw).unwrap();
        assert_eq!(result.as_slice(), b"password_new");

        let pass = db.get("user", pw).unwrap();
        assert_eq!(b"password", pass.as_slice());
    }

    #[test]
    fn test_serialize_deserialize_encrypted() {
        let mut db = DatabaseEncrypted::new(b"test").unwrap();
        db.insert("user", b"password", b"test").unwrap();
        db.insert("user2", b"password", b"test").unwrap();

        let serialized = db.serialize_encrypted(b"test").unwrap();
        let db = DatabaseEncrypted::new_deserialize_encrypted(&serialized, b"test").unwrap();

        let pass = db.get("user", b"test").unwrap();
        assert_eq!(b"password", pass.as_slice())
    }

    #[test]
    fn test_csv() {
        let pw = b"test";
        let mut db = DatabaseEncrypted::new(pw).unwrap();
        db.insert("user", b"password", pw).unwrap();
        db.insert("user2", b"password,,,broken?", pw).unwrap();
        db.export_to_csv("db_encrypted_csv_test00000000.csv", pw)
            .unwrap();
        let mut db = DatabaseEncrypted::new(pw).unwrap();
        db.insert_from_csv("db_encrypted_csv_test00000000.csv", pw)
            .unwrap();
        std::fs::remove_file("db_encrypted_csv_test00000000.csv").unwrap();

        let pass = db.get("user", pw).unwrap();
        assert_eq!(b"password", pass.as_slice());
        let pass = db.get("user2", pw).unwrap();
        assert_eq!(b"password,,,broken?", pass.as_slice());
    }
}
