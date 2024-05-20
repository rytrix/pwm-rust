use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub mod aes_wrapper;
pub mod chacha20_wrapper;

pub mod default {
    use crate::hash::HashResult;

    use super::{
        aes_wrapper::{aes_decrypt, aes_encrypt},
        EncryptionError, EncryptionResult,
    };

    pub fn encrypt(
        plaintext: &[u8],
        key: &HashResult,
    ) -> Result<EncryptionResult, EncryptionError> {
        #[cfg(feature = "use-aes-default")]
        let result = aes_encrypt(plaintext, key);
        #[cfg(feature = "use-chacha20-default")]
        let result = super::chacha20_wrapper::ecrypt(plaintext, key);

        result
    }

    pub fn decrypt(
        ciphertext: &EncryptionResult,
        key: &HashResult,
    ) -> Result<EncryptionResult, EncryptionError> {
        #[cfg(feature = "use-aes-default")]
        let result = aes_decrypt(ciphertext, key);
        #[cfg(feature = "use-chacha20-default")]
        let result = super::chacha20_wrapper::decrypt(ciphertext, key);

        result
    }
}

#[derive(Debug)]
pub struct EncryptionError {
    error: String,
}

impl EncryptionError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.as_ref())
    }
}

impl std::error::Error for EncryptionError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncryptionResult {
    data: Vec<u8>,
}

impl EncryptionResult {
    pub fn new(data: Vec<u8>) -> Result<EncryptionResult, EncryptionError> {
        Ok(Self { data })
    }

    pub fn as_ref(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data.as_slice()
    }

    pub fn get_salt_slice(&self) -> &[u8] {
        &self.data[self.data.len() - 32..]
    }

    pub fn get_crypt_slice(&self) -> &[u8] {
        &self.data[..self.data.len() - 32]
    }
}

impl Zeroize for EncryptionResult {
    fn zeroize(&mut self) {
        self.data.zeroize();
    }
}

impl Drop for EncryptionResult {
    fn drop(&mut self) {
        self.zeroize();
    }
}
