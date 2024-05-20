use crate::encryption::{EncryptionError, EncryptionResult};
use crate::hash::HashResult;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Key,
};

impl From<aes_gcm::Error> for EncryptionError {
    fn from(_value: aes_gcm::Error) -> Self {
        Self::new("Failed AES, invalid key")
    }
}

// Salt is appended to the end of the cipher
pub fn aes_gcm_encrypt(
    plaintext: &[u8],
    hash_result: &HashResult,
) -> Result<EncryptionResult, EncryptionError> {
    let key = Key::<Aes256Gcm>::from_slice(hash_result.get_hash());

    let cipher = Aes256Gcm::new(&key);

    // if doing absurd number of random numbers over 4 million consider siv
    let nonce = Aes256Gcm::generate_nonce(&mut aead::OsRng); // 96-bits; unique per message
    let mut ciphertext = cipher.encrypt(&nonce, plaintext)?;
    ciphertext.extend_from_slice(nonce.as_slice());
    ciphertext.extend_from_slice(hash_result.get_salt());

    Ok(EncryptionResult::new(ciphertext)?)
}

pub fn aes_gcm_decrypt(
    ciphertext: &EncryptionResult,
    key: &HashResult,
) -> Result<EncryptionResult, EncryptionError> {
    let key = Key::<Aes256Gcm>::from_slice(key.get_hash());

    let ciphertext = ciphertext.get_crypt_slice();

    let cipher = Aes256Gcm::new(&key);
    let nonce = &ciphertext[ciphertext.len() - 12..];

    let plaintext = cipher.decrypt(nonce.into(), &ciphertext[..ciphertext.len() - 12])?;

    Ok(EncryptionResult::new(plaintext)?)
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

        let ciphertext = aes_gcm_encrypt(plaintext, &hash).unwrap();

        let plaintext_result = aes_gcm_decrypt(&ciphertext, &hash).unwrap();

        let matching = plaintext
            .as_ref()
            .iter()
            .zip(plaintext_result.as_ref())
            .filter(|&(a, b)| a == b)
            .count();

        assert!(matching == plaintext.len())
    }
}
