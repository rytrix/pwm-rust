use crate::hash::argon2_wrapper::{argon2_hash_password, argon2_hash_password_with_salt};

use crate::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult};
use crate::zeroize::Zeroizing;

pub fn encrypt_file(file: String, output: Option<String>, password: &[u8]) -> Result<(), std::io::Error> {
    let hash = match argon2_hash_password(password) {
        Ok(hash) => hash,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let contents = Zeroizing::new(std::fs::read(&file)?);
    let cipher_contents = match aes_gcm_encrypt(&hash, contents.as_slice()) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let output = match output {
        Some(output) => output,
        None => file,
    };

    std::fs::write(output, cipher_contents.as_slice())?;

    Ok(())
}

pub fn decrypt_file(file: String, output: Option<String>, password: &[u8]) -> Result<(), std::io::Error> {
    let contents = AesResult::new(std::fs::read(&file)?)?;

    let hash = match argon2_hash_password_with_salt(password, contents.get_salt_slice())
    {
        Ok(hash) => hash,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let cipher_contents = match aes_gcm_decrypt(hash.get_hash(), &contents) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let output = match output {
        Some(output) => output,
        None => file,
    };

    std::fs::write(output, cipher_contents.as_slice())?;

    Ok(())
}