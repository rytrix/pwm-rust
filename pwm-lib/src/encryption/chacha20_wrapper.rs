use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305,
};

use crate::hash::HashResult;

use super::{EncryptionError, EncryptionResult};

pub fn chacha20_encrypt(plaintext: &[u8], key: &HashResult) -> Result<EncryptionResult, EncryptionError> {
    let cipher = XChaCha20Poly1305::new(key.get_hash().into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng); // 192-bits; unique per message
    let mut ciphertext = cipher.encrypt(&nonce, plaintext)?;
    ciphertext.extend_from_slice(nonce.as_slice());
    ciphertext.extend_from_slice(key.get_salt());

    EncryptionResult::new(ciphertext)
}

pub fn chacha20_decrypt(ciphertext: &EncryptionResult, key: &HashResult) -> Result<EncryptionResult, EncryptionError> {
    let cipher = XChaCha20Poly1305::new(key.get_hash().into());
    let ciphertext = ciphertext.get_crypt_slice();

    let nonce = &ciphertext[ciphertext.len() - 24..];
    let ciphertext = &ciphertext[..ciphertext.len() - 24];

    let plaintext = cipher.decrypt(nonce.into(), ciphertext)?;

    EncryptionResult::new(plaintext)
}

#[cfg(test)]
mod tests {
    use crate::hash::pbkdf2_wrapper::pbkdf2_hash_password;

    use super::{chacha20_decrypt, chacha20_encrypt};

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = b"hello world 123";
        let password = b"00000000000000000000000000000000";

        let hash = pbkdf2_hash_password(password).unwrap();

        let ciphertext = chacha20_encrypt(plaintext, &hash).unwrap();
        let decrypted = chacha20_decrypt(&ciphertext, &hash).unwrap();

        // eprintln!("{:?}\n{:?}", plaintext, decrypted);
        assert_eq!(plaintext, decrypted.as_slice());
    }
}
