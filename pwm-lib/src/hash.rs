pub mod argon2_wrapper;
pub mod pbkdf2_wrapper;
pub mod scrypt_wrapper;
pub mod sha_wrapper;

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

        randomize_slice(&mut result.salt);

        return result;
    }

    pub fn new_with_salt(salt: &[u8]) -> Result<HashResult, HashError> {
        if salt.len() != 32 {
            return Err(HashError::new("Invalid salt length, expected 32"));
        }

        let mut result = HashResult {
            salt: [0; 32],
            hash: [0; 32],
        };

        result.salt.copy_from_slice(salt);

        Ok(result)
    }

    pub fn get_salt(&self) -> &[u8] {
        &self.salt
    }

    pub fn get_hash(&self) -> &[u8] {
        &self.hash
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
    pub fn new(msg: &str) -> Self {
        Self {
            error: msg.to_string(),
        }
    }
}

impl std::fmt::Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.as_ref())
    }
}

impl std::error::Error for HashError {}

pub fn randomize_slice(data: &mut [u8]) {
    aead::OsRng::fill_bytes(&mut aead::OsRng, data);
}

// True if the same
pub fn compare_hash(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for (x, y) in a.iter().zip(b.iter()) {
        if *x != *y {
            return false;
        }
    }

    true
}
