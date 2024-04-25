use crate::salt::{SaltError, SaltResult};

use scrypt::{scrypt, Params};

fn scrypt_default_args() -> Result<Params, SaltError> {
    let params = Params::new(
        Params::RECOMMENDED_LOG_N,
        Params::RECOMMENDED_R,
        Params::RECOMMENDED_P,
        Params::RECOMMENDED_LEN,
    );

    let params = match params {
        Ok(params) => params,
        Err(error) => {
            return Err(SaltError::new(error.to_string()));
        }
    };

    Ok(params)
}

pub fn scrypt_hash_password(password: &[u8]) -> Result<SaltResult, SaltError> {
    let mut result = SaltResult::new();

    let params = scrypt_default_args()?;

    let scrypt_result = scrypt(password, &result.salt, &params, &mut result.hash);
    match scrypt_result {
        Ok(()) => {}
        Err(error) => {
            return Err(SaltError::new(error.to_string()));
        }
    }

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
