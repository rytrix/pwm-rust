use std::io::Write;

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
