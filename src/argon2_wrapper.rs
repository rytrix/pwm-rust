use argon2::{Algorithm, Argon2, Params};
use crate::salt::{SaltError, SaltResult};

fn argon2_default<'a>() -> Argon2<'a> {
    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(Params::DEFAULT_M_COST, 4, 4, None).unwrap(),
    );

    return argon2;
}

#[allow(unused)]
pub fn argon2_hash_password(password: &[u8]) -> Result<SaltResult, SaltError> {
    let argon2 = argon2_default();
    let mut result = SaltResult::new();

    let argon2_result = argon2.hash_password_into(password, &result.salt, &mut result.hash);
    match argon2_result {
        Ok(()) => {}
        Err(error) => {
            return Err(SaltError::new(error.to_string()))
        }
    }

    Ok(result)
}
