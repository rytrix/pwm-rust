use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

use crate::hash::{HashError, HashResult};

// TODO how the heck do I find the library default for number of iterations
// Updated April 25 of 2024
static PBKDF2_DEFAULT_N: u32 = 600_000;

pub fn pbkdf2_hash_password_into(password: &[u8], result: &mut HashResult) -> Result<(), HashError> {
    pbkdf2_hmac::<Sha256>(password, &result.salt, PBKDF2_DEFAULT_N, &mut result.hash);
    Ok(())
}

pub fn pbkdf2_hash_password(password: &[u8]) -> Result<HashResult, HashError> {
    let mut result = HashResult::new();
    pbkdf2_hash_password_into(password, &mut result)?;

    Ok(result)
}

pub fn pbkdf2_hash_password_with_salt(password: &[u8], salt: &[u8]) -> Result<HashResult, HashError> {
    let mut result = HashResult::new_with_salt(salt)?;
    pbkdf2_hash_password_into(password, &mut result)?;

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::pbkdf2_hash_password;

    #[test]
    fn test_pbkdf2_for_crash() {
        let password = b"password123";
        let _ = pbkdf2_hash_password(password).unwrap();
    }
}