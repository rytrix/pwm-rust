use std::io::Write;

// use pwm_lib::scrypt_wrapper::scrypt_hash_password;
use pwm_lib::hash::argon2_wrapper::{argon2_hash_password, argon2_hash_password_with_salt};
// use pwm_lib::hash::pbkdf2_wrapper::pbkdf2_hash_password;

use pwm_lib::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult};
use pwm_lib::zeroize::Zeroizing;

pub fn request_password(prompt: &str) -> Result<Zeroizing<String>, std::io::Error> {
    print!("{}", prompt);
    std::io::stdout().flush()?;

    Ok(Zeroizing::new(rpassword::read_password()?))
}

pub fn password_confirmation() -> Result<Zeroizing<String>, std::io::Error> {
    let password1 = request_password("Enter your password")?;
    let password2 = request_password("Enter your password again")?;

    if !password1.eq(&password2) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "passwords do not match",
        ));
    }

    Ok(password1)
}

pub fn encrypt_file(file: String, output: Option<String>) -> Result<(), std::io::Error> {
    let password = password_confirmation()?;

    let hash = match argon2_hash_password(password.as_bytes()) {
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

pub fn decrypt_file(file: String, output: Option<String>) -> Result<(), std::io::Error> {
    let password = request_password("Enter your password")?;

    let contents = AesResult::new(std::fs::read(&file)?)?;

    let hash = match argon2_hash_password_with_salt(password.as_bytes(), contents.get_salt_slice())
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