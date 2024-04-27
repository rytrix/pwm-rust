use crate::hash::{HashError, HashResult};
use argon2::{Algorithm, Argon2, Params};

// Updated April 25 of 2024
fn argon2_default<'a>() -> Argon2<'a> {
    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(Params::DEFAULT_M_COST, 4, 4, None).unwrap(),
    );

    return argon2;
}

pub fn argon2_hash_password_into(password: &[u8], result: &mut HashResult) -> Result<(), HashError> {
    let argon2 = argon2_default();

    let argon2_result = argon2.hash_password_into(password, &result.salt, &mut result.hash);
    match argon2_result {
        Ok(()) => {}
        Err(error) => return Err(HashError::new(error.to_string().as_str())),
    }

    #[cfg(feature = "pepper")]
    super::pepper::pepper_hash(&mut result.hash);

    Ok(())
}

pub fn argon2_hash_password(password: &[u8]) -> Result<HashResult, HashError> {
    let mut result = HashResult::new();
    argon2_hash_password_into(password, &mut result)?;
    
    Ok(result)
}
pub fn argon2_hash_password_with_salt(password: &[u8], salt: &[u8]) -> Result<HashResult, HashError> {
    let mut result = HashResult::new_with_salt(salt)?;
    argon2_hash_password_into(password, &mut result)?;

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::argon2_hash_password;

    #[test]
    fn test_argon2_for_crash() {
        let password = b"password123";
        let _ = argon2_hash_password(password).unwrap();
    }
}