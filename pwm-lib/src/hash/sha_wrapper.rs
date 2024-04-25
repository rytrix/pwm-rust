use sha2::{Digest, Sha256, Sha512};

use super::HashError;

pub fn sha256_hash(data: &[u8], output: &mut [u8]) -> Result<(), HashError> {
    if output.len() != 32 {
        return Err(HashError::new("invalid output length"));
    }

    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize_into(output.into());

    Ok(())
}

pub fn sha512_hash(data: &[u8], output: &mut [u8]) -> Result<(), HashError> {
    if output.len() != 64 {
        return Err(HashError::new("invalid output length"));
    }

    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize_into(output.into());

    Ok(())
}

