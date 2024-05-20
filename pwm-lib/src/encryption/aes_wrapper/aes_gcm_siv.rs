use crate::{
    encryption::{EncryptionError, EncryptionResult},
    hash::HashResult,
};
use aes_gcm_siv::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256GcmSiv,
};

impl From<aes_gcm::Error> for EncryptionError {
    fn from(_value: aes_gcm::Error) -> Self {
        Self::new("Failed encryption, invalid key")
    }
}

// Salt is appended to the end of the cipher
pub fn aes_gcm_siv_encrypt(
    plaintext: &[u8],
    hash_result: &HashResult,
) -> Result<EncryptionResult, EncryptionError> {
    let cipher = Aes256GcmSiv::new(hash_result.get_hash().into());
    let nonce = Aes256GcmSiv::generate_nonce(&mut aead::OsRng); // 96-bits; unique per message

    let mut ciphertext = cipher.encrypt(&nonce, plaintext)?;
    ciphertext.extend_from_slice(nonce.as_slice());
    ciphertext.extend_from_slice(hash_result.get_salt());

    return Ok(EncryptionResult { data: ciphertext });
}

pub fn aes_gcm_siv_decrypt(
    ciphertext: &EncryptionResult,
    key: &HashResult,
) -> Result<EncryptionResult, EncryptionError> {
    let cipher = Aes256GcmSiv::new(key.get_hash().into());
    let ciphertext = ciphertext.get_crypt_slice();
    let nonce = &ciphertext[ciphertext.len() - 12..];

    let plaintext = cipher.decrypt(nonce.into(), &ciphertext[..ciphertext.len() - 12])?;

    return Ok(EncryptionResult { data: plaintext });
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

        let ciphertext = aes_gcm_siv_encrypt(plaintext, &hash).unwrap();

        let plaintext_result = aes_gcm_siv_decrypt(&ciphertext, &hash).unwrap();

        let matching = plaintext
            .as_ref()
            .iter()
            .zip(plaintext_result.as_ref())
            .filter(|&(a, b)| a == b)
            .count();

        assert!(matching == plaintext.len())
    }
}
