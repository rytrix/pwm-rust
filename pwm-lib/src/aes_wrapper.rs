use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Key,
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::hash::HashResult;

#[derive(Serialize, Deserialize)]
pub struct AesResult {
    data: Vec<u8>,
}

impl AesResult {
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

pub fn random_key() -> zeroize::Zeroizing<[u8; 32]> {
    let key = Aes256Gcm::generate_key(aead::OsRng);
    return Zeroizing::new(key.into());
}

pub fn aes_gcm_encrypt(
    hash_result: &HashResult,
    plaintext: &[u8],
) -> Result<AesResult, aes_gcm::Error> {
    let key = Key::<Aes256Gcm>::from_slice(hash_result.get_hash());

    let cipher = Aes256Gcm::new(&key);

    // TODO if doing absurd number of random numbers over 4 million consider siv
    let nonce = Aes256Gcm::generate_nonce(&mut aead::OsRng); // 96-bits; unique per message
    let mut ciphertext = cipher.encrypt(&nonce, plaintext)?;
    ciphertext.extend_from_slice(nonce.as_slice());
    ciphertext.extend_from_slice(hash_result.get_salt());

    return Ok(AesResult { data: ciphertext });
}

pub fn aes_gcm_decrypt(key: &[u8], ciphertext: &AesResult) -> Result<AesResult, aes_gcm::Error> {
    let key = Key::<Aes256Gcm>::from_slice(key);

    let ciphertext = ciphertext.get_crypt_slice();

    let cipher = Aes256Gcm::new(&key);
    let nonce = &ciphertext[ciphertext.len() - 12..];

    let plaintext = cipher.decrypt(nonce.into(), &ciphertext[..ciphertext.len() - 12])?;

    return Ok(AesResult { data: plaintext });
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
