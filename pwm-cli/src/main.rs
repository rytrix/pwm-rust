use std::io::Write;

// use pwm_lib::scrypt_wrapper::scrypt_hash_password;
use pwm_lib::hash::argon2_wrapper::{argon2_hash_password, argon2_hash_password_with_salt};
// use pwm_lib::hash::pbkdf2_wrapper::pbkdf2_hash_password;

use pwm_lib::aes_wrapper::{aes_gcm_decrypt, aes_gcm_encrypt, AesResult};
use pwm_lib::zeroize::Zeroizing;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// File to encrypt
    #[arg(short, long, value_name = "file")]
    encrypt: Option<String>,

    /// File to decrypt
    #[arg(short, long, value_name = "file")]
    decrypt: Option<String>,

    /// Vault to open
    #[arg(short, long, value_name = "file")]
    vault: Option<String>,

    /// Create a vault
    #[arg(short, long, value_name = "boolean", default_value_t = false)]
    create: bool,

    /// Output file
    #[arg(short, long, value_name = "file")]
    out: Option<String>,
}

fn encrypt_file(file: String, output: Option<String>) -> Result<(), std::io::Error> {
    print!("Enter your password");
    std::io::stdout().flush()?;
    let password1 = Zeroizing::new(rpassword::read_password()?);
    print!("Enter your password again");
    std::io::stdout().flush()?;
    let password2 = Zeroizing::new(rpassword::read_password()?);

    if !password1.eq(&password2) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "passwords do not match",
        ));
    }

    let hash = match argon2_hash_password(password1.as_bytes()) {
        Ok(hash) => hash,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let contents = Zeroizing::new(std::fs::read(&file)?);
    let cipher_contents = match aes_gcm_encrypt(&hash, contents.as_slice()) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let output = match output {
        Some(output) => output,
        None => file,
    };

    std::fs::write(output, cipher_contents.as_slice())?;

    Ok(())
}

fn decrypt_file(file: String, output: Option<String>) -> Result<(), std::io::Error> {
    print!("Enter your password");
    std::io::stdout().flush()?;
    let password1 = Zeroizing::new(rpassword::read_password()?);
    print!("Enter your password again");
    std::io::stdout().flush()?;
    let password2 = Zeroizing::new(rpassword::read_password()?);

    if !password1.eq(&password2) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "passwords do not match",
        ));
    }

    let contents = AesResult::new(std::fs::read(&file)?);

    let hash = match argon2_hash_password_with_salt(password1.as_bytes(), contents.get_salt_slice())
    {
        Ok(hash) => hash,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let cipher_contents = match aes_gcm_decrypt(hash.get_hash(), &contents) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ))
        }
    };

    let output = match output {
        Some(output) => output,
        None => file,
    };

    std::fs::write(output, cipher_contents.as_slice())?;

    Ok(())
}

fn main() {
    let args = Args::parse();

    if args.decrypt.is_none() && args.vault.is_none() && !args.create {
        // Encrypt
        if let Some(name) = args.encrypt {
            println!("Encrypting file {}", name);
            encrypt_file(name, args.out).unwrap();
        }
    } else if args.encrypt.is_none() && args.vault.is_none() && !args.create {
        // Decrypt
        if let Some(name) = args.decrypt {
            println!("Decrypting file {}", name);
            decrypt_file(name, args.out).unwrap();
        }
    } else if args.encrypt.is_none() && args.decrypt.is_none() && args.create {
        // Vault
        if args.out.is_some() {
            println!("ignoring out parameter for vault");
        }
        if let Some(name) = args.vault {
            println!("vault {}", name);
        }
    } else if args.encrypt.is_none() && args.decrypt.is_none() && args.vault.is_none() {
        // New Vault
        if args.out.is_some() {
            println!("ignoring out parameter for create");
        }
        if args.create {
            println!("creating vault");
        }
    } else {
        println!("to many arguments provided, only provide encrypt, decrypt or vault");
    }

    // let password = Zeroizing::new(*b"hunter42"); // Bad password; don't actually use!
    // let plaintext = Zeroizing::new(*b"hello world");

    // let hash = pbkdf2_hash_password(password.as_ref()).unwrap();

    // let ciphertext = aes_gcm_encrypt(&hash, plaintext.as_ref()).unwrap();

    // let plaintext = aes_gcm_decrypt(hash.get_hash(), &ciphertext).unwrap();

    // println!(
    //     "{}",
    //     String::from_utf8(plaintext.as_ref().to_vec()).unwrap()
    // );
}
