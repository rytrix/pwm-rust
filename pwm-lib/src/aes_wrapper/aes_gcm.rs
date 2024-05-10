use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Key,
};
use crate::hash::HashResult;
use super::{AesError, AesResult};

impl From<aes_gcm::Error> for AesError {
    fn from(_value: aes_gcm::Error) -> Self {
        Self::new("Failed AES, invalid key")
    }
}

// Salt is appended to the end of the cipher
pub fn aes_gcm_encrypt(
    hash_result: &HashResult,
    plaintext: &[u8],
) -> Result<AesResult, aes_gcm::Error> {
    let key = Key::<Aes256Gcm>::from_slice(hash_result.get_hash());

    let cipher = Aes256Gcm::new(&key);

    // if doing absurd number of random numbers over 4 million consider siv
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