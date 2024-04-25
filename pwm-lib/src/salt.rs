use aead::rand_core::RngCore;
use zeroize::Zeroize;

pub struct SaltResult {
    pub salt: [u8; 32],
    pub hash: [u8; 32],
}

impl SaltResult {
    // Implicitly generates a random salt using aead::OsRng
    pub fn new() -> SaltResult {
        let mut result = SaltResult {
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

impl Drop for SaltResult {
    fn drop(&mut self) {
        self.salt.zeroize();
        self.hash.zeroize();
    }
}

#[derive(Debug)]
pub struct SaltError {
    error: String,
}

impl SaltError {
    pub fn new(msg: String) -> Self {
        Self { error: msg }
    }
}

impl std::fmt::Display for SaltError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.as_ref())
    }
}

impl std::error::Error for SaltError {}
