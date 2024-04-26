mod crypt_file;

use clap::Parser;

use crate::crypt_file::{decrypt_file, encrypt_file};

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

fn main() {
    let args = Args::parse();

    if args.decrypt.is_none() && args.vault.is_none() && !args.create {
        // Encrypt
        if let Some(name) = args.encrypt {
            println!("Encrypting file {}", name);
            if let Err(error) = encrypt_file(name, args.out) {
                println!("Error: {}", error.to_string());
            }
        }
    } else if args.encrypt.is_none() && args.vault.is_none() && !args.create {
        // Decrypt
        if let Some(name) = args.decrypt {
            println!("Decrypting file {}", name);
            if let Err(_error) = decrypt_file(name, args.out) {
                println!("Invalid password");
            }
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
}
