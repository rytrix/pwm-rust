use crate::hash::{HashError, HashResult};

use scrypt::{scrypt, Params};

// Updated April 25 of 2024
fn scrypt_default_args() -> Result<Params, HashError> {
    let params = Params::new(
        Params::RECOMMENDED_LOG_N,
        Params::RECOMMENDED_R,
        Params::RECOMMENDED_P,
        Params::RECOMMENDED_LEN,
    );

    let params = match params {
        Ok(params) => params,
        Err(error) => {
            return Err(HashError::new(error.to_string().as_str()));
        }
    };

    Ok(params)
}

pub fn scrypt_hash_password_into(
    password: &[u8],
    result: &mut HashResult,
) -> Result<(), HashError> {
    let params = scrypt_default_args()?;

    let scrypt_result = scrypt(password, &result.salt, &params, &mut result.hash);
    match scrypt_result {
        Ok(()) => {}
        Err(error) => {
            return Err(HashError::new(error.to_string().as_str()));
        }
    }

    Ok(())
}

pub fn scrypt_hash_password(password: &[u8]) -> Result<HashResult, HashError> {
    let mut result = HashResult::new();
    scrypt_hash_password_into(password, &mut result)?;

    return Ok(result);
}

pub fn scrypt_hash_password_with_salt(password: &[u8], salt: &[u8]) -> Result<HashResult, HashError> {
    let mut result = HashResult::new_with_salt(salt)?;
    scrypt_hash_password_into(password, &mut result)?;

    return Ok(result);
}

#[cfg(test)]
mod test {
    use super::scrypt_hash_password;

    #[test]
    fn test_pbkdf2_for_crash() {
        let password = b"password123";
        let _ = scrypt_hash_password(password).unwrap();
    }
}
