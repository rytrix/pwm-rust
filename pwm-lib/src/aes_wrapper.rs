use zeroize::Zeroize;
use serde::{Deserialize, Serialize};

use crate::hash::HashResult;

#[cfg(not(feature = "use-aes-gcm-siv"))]
mod aes_gcm;
#[cfg(feature = "use-aes-gcm-siv")]
mod aes_gcm_siv;

pub fn aes_gcm_encrypt(
    hash_result: &HashResult,
    plaintext: &[u8],
) -> Result<AesResult, AesError> {
    #[cfg(feature = "use-aes-gcm-siv")]
    let result = aes_gcm_siv::aes_gcm_encrypt(hash_result, plaintext)?;
    #[cfg(not(feature = "use-aes-gcm-siv"))]
    let result = aes_gcm::aes_gcm_encrypt(hash_result, plaintext)?;
    Ok(result)
}

pub fn aes_gcm_decrypt(key: &[u8], ciphertext: &AesResult) -> Result<AesResult, AesError> {
    #[cfg(feature = "use-aes-gcm-siv")]
    let result = aes_gcm_siv::aes_gcm_decrypt(key, ciphertext)?;
    #[cfg(not(feature = "use-aes-gcm-siv"))]
    let result = aes_gcm::aes_gcm_decrypt(key, ciphertext)?;
    Ok(result)
}

#[derive(Debug)]
pub struct AesError {
    error: String,
}

impl AesError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
        }
    }
}

impl std::fmt::Display for AesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.as_ref())
    }
}

impl std::error::Error for AesError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AesResult {
    data: Vec<u8>,
}

impl AesResult {
    pub fn new(data: Vec<u8>) -> Result<Self, std::io::Error> {
        if data.len() <= 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Data is likely not encrypted",
            ))
        }
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

impl Zeroize for AesResult {
    fn zeroize(&mut self) {
        self.data.zeroize();
    }
}

impl Drop for AesResult {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hash::pbkdf2_wrapper::pbkdf2_hash_password;

    #[test]
    fn test_aes256_for_crash() {
        let password = b"hunter42";
        let plaintext = b"hello world";

        let hash = pbkdf2_hash_password(password).unwrap();

        let ciphertext = aes_gcm_encrypt(&hash, plaintext).unwrap();

        let plaintext_result = aes_gcm_decrypt(hash.get_hash(), &ciphertext).unwrap();

        let matching = plaintext
            .as_ref()
            .iter()
            .zip(plaintext_result.as_ref())
            .filter(|&(a, b)| a == b)
            .count();

        assert!(matching == plaintext.len())
    }
}
