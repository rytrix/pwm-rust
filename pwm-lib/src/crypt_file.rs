use crate::encryption::default::{decrypt, encrypt};
use crate::encryption::EncryptionResult;
use crate::hash::argon2_wrapper::{argon2_hash_password, argon2_hash_password_with_salt};

use crate::zeroize::Zeroizing;

pub fn encrypt_file(
    file: String,
    output: Option<String>,
    password: &[u8],
) -> Result<(), std::io::Error> {
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
    let cipher_contents = match encrypt(contents.as_slice(), &hash) {
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

pub fn decrypt_file(
    file: String,
    output: Option<String>,
    password: &[u8],
) -> Result<(), std::io::Error> {
    let contents = match EncryptionResult::new(std::fs::read(&file)?) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error.to_string(),
            ))
        }
    };

    let hash = match argon2_hash_password_with_salt(password, contents.get_salt_slice()) {
        Ok(hash) => hash,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let cipher_contents = match decrypt(&contents, &hash) {
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
