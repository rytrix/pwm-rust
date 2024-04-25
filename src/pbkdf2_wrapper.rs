use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

use crate::salt::{SaltError, SaltResult};

// TODO how the heck do I find the library default for number of iterations
static PBKDF2_DEFAULT_N: u32 = 600_000;

#[allow(unused)]
pub fn pbkdf2_hash_password(password: &[u8]) -> Result<SaltResult, SaltError> {
    let mut result = SaltResult::new();

    pbkdf2_hmac::<Sha256>(password, &result.salt, PBKDF2_DEFAULT_N, &mut result.hash);

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
