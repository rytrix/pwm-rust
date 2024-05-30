use passwords::PasswordGenerator;

pub fn random_password(length: usize) -> Result<String, &'static str> {
    let pg = PasswordGenerator {
        length,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: false,
        strict: true,
    };

    match pg.generate_one() {
        Ok(value) => Ok(value),
        Err(error) => Err(error),
    }
}
