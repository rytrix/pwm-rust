use passwords::PasswordGenerator;

pub fn random_password(length: usize) -> String {
    let pg = PasswordGenerator {
        length,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: true,
    };

    match pg.generate_one() {
        Ok(value) => value,
        Err(value) => value.to_string(),
    }
}
