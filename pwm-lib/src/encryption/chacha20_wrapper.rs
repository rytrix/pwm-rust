use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305,
};

use crate::hash::HashResult;

use super::{EncryptionError, EncryptionResult};

pub fn encrypt(plaintext: &[u8], key: &HashResult) -> Result<EncryptionResult, EncryptionError> {
    let cipher = XChaCha20Poly1305::new(key.get_hash().into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng); // 192-bits; unique per message
    let mut ciphertext = cipher.encrypt(&nonce, plaintext)?;
    ciphertext.extend_from_slice(nonce.as_slice());

    EncryptionResult::new(ciphertext)
}

pub fn decrypt(ciphertext: &[u8], key: &HashResult) -> Result<EncryptionResult, EncryptionError> {
    let cipher = XChaCha20Poly1305::new(key.get_hash().into());

    let nonce = &ciphertext[ciphertext.len() - 24..];
    let ciphertext = &ciphertext[..ciphertext.len() - 24];

    let plaintext = cipher.decrypt(nonce.into(), ciphertext)?;

    EncryptionResult::new(plaintext)
}

#[cfg(test)]
mod tests {
    use crate::hash::pbkdf2_wrapper::pbkdf2_hash_password;

    use super::{decrypt, encrypt};

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = b"hello world 123";
        let password = b"00000000000000000000000000000000";

        let hash = pbkdf2_hash_password(password).unwrap();

        let ciphertext = encrypt(plaintext, &hash).unwrap();
        let decrypted = decrypt(ciphertext.as_slice(), &hash).unwrap();

        // eprintln!("{:?}\n{:?}", plaintext, decrypted);
        assert_eq!(plaintext, decrypted.as_slice());
    }
}
