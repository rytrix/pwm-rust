pub mod scrypt_wrapper;
pub mod argon2_wrapper;
pub mod pbkdf2_wrapper;

use aead::rand_core::RngCore;
use zeroize::Zeroize;

pub struct HashResult {
    salt: [u8; 32],
    hash: [u8; 32],
}

impl HashResult {
    // Implicitly generates a random salt using aead::OsRng
    pub fn new() -> HashResult {
        let mut result = HashResult {
            salt: [0; 32],
            hash: [0; 32],
        };

        result.randomize_salt();

        return result;
    }

    pub fn get_salt(&self) -> &[u8] {
        &self.salt
    }

    pub fn get_hash(&self) -> &[u8] {
        &self.hash
    }

    fn randomize_salt(&mut self) {
        aead::OsRng::fill_bytes(&mut aead::OsRng, &mut self.salt);
    }
}

impl Drop for HashResult {
    fn drop(&mut self) {
        self.salt.zeroize();
        self.hash.zeroize();
    }
}

#[derive(Debug)]
pub struct HashError {
    error: String,
}

impl HashError {
    pub fn new(msg: String) -> Self {
        Self { error: msg }
    }
}

impl std::fmt::Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.as_ref())
    }
}

impl std::error::Error for HashError {}