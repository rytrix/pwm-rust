use super::{EncryptionError, EncryptionResult};
use crate::hash::HashResult;

#[cfg(not(feature = "use-aes-gcm-siv"))]
mod aes_gcm;
#[cfg(feature = "use-aes-gcm-siv")]
mod aes_gcm_siv;

pub fn aes_encrypt(
    plaintext: &[u8],
    hash: &HashResult,
) -> Result<EncryptionResult, EncryptionError> {
    #[cfg(feature = "use-aes-gcm-siv")]
    let result = aes_gcm_siv::aes_gcm_siv_encrypt(plaintext, hash)?;
    #[cfg(not(feature = "use-aes-gcm-siv"))]
    let result = aes_gcm::aes_gcm_encrypt(plaintext, hash)?;
    Ok(result)
}

pub fn aes_decrypt(
    ciphertext: &EncryptionResult,
    hash: &HashResult,
) -> Result<EncryptionResult, EncryptionError> {
    #[cfg(feature = "use-aes-gcm-siv")]
    let result = aes_gcm_siv::aes_gcm_siv_decrypt(ciphertext, hash)?;
    #[cfg(not(feature = "use-aes-gcm-siv"))]
    let result = aes_gcm::aes_gcm_decrypt(ciphertext, hash)?;
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hash::pbkdf2_wrapper::pbkdf2_hash_password;

    #[test]
    fn test_aes256_for_crash() {
        let password = b"reallysimplepassword";
        let plaintext = b"poggers";

        let hash = pbkdf2_hash_password(password).unwrap();

        let ciphertext = aes_encrypt(plaintext, &hash).unwrap();

        let plaintext_result = aes_decrypt(&ciphertext, &hash).unwrap();

        let matching = plaintext
            .as_ref()
            .iter()
            .zip(plaintext_result.as_ref())
            .filter(|&(a, b)| a == b)
            .count();

        assert!(matching == plaintext.len())
    }
}
