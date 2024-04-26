mod crypt_file;
mod vault;

use crate::{crypt_file::{decrypt_file, encrypt_file}, vault::Vault};

use clap::{ArgAction, Parser};

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
    #[arg(short, long, value_name = "boolean", action = ArgAction::SetTrue)]
    create: bool,

    /// Output file
    #[arg(short, long, value_name = "file")]
    out: Option<String>,
}

#[tokio::main]
async fn main() {
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
            if let Err(error) = decrypt_file(name, args.out) {
                println!("Error: {}", error);
            }
        }
    } else if args.encrypt.is_none() && args.decrypt.is_none() && !args.create {
        // Vault
        if args.out.is_some() {
            println!("ignoring out parameter for vault");
        }
        if let Some(name) = args.vault {
            println!("Loading a vault from the file {}", name);
            let mut vault = match Vault::new_from_file(name.as_str()).await {
                Ok(vault) => vault,
                Err(error) => {
                    println!("Error: {}", error.to_string());
                    return;
                }
            };

            vault.run().await;
        }
    } else if args.encrypt.is_none() && args.decrypt.is_none() && args.vault.is_none() {
        // New Vault
        if args.out.is_some() {
            println!("ignoring out parameter for create");
        }
        if args.create {
            println!("Creating a new vault");
            let mut vault = match Vault::new().await {
                Ok(vault) => vault,
                Err(error) => {
                    println!("Error: {}", error.to_string());
                    return;
                }
            };

            vault.run().await;
        }
    } else {
        println!("to many arguments provided, only provide encrypt, decrypt, vault or create");
    }

    println!("exiting");
}
