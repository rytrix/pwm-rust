mod password;
mod vault;

use crate::{password::{password_confirmation, request_password}, vault::Vault};
use pwm_lib::crypt_file::{decrypt_file, encrypt_file};

use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
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

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    if args.decrypt.is_none() && args.vault.is_none() && !args.create {
        // Encrypt
        if let Some(name) = args.encrypt {
            println!("Encrypting file {}", name);
            let password = password_confirmation()?;
            if let Err(error) = encrypt_file(name, args.out, password.as_bytes()) {
                println!("Error: {}", error.to_string());
            }
        }
    } else if args.encrypt.is_none() && args.vault.is_none() && !args.create {
        // Decrypt
        if let Some(name) = args.decrypt {
            println!("Decrypting file {}", name);
            let password = request_password("Enter your password")?;
            if let Err(error) = decrypt_file(name, args.out, password.as_bytes()) {
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
            let mut vault = match Vault::new_from_file(name.as_str()) {
                Ok(vault) => vault,
                Err(error) => {
                    println!("Error: {}", error.to_string());
                    return Ok(());
                }
            };

            vault.run();
        }
    } else if args.encrypt.is_none() && args.decrypt.is_none() && args.vault.is_none() {
        // New Vault
        if args.out.is_some() {
            println!("ignoring out parameter for create");
        }
        if args.create {
            println!("Creating a new vault");
            let mut vault = match Vault::new() {
                Ok(vault) => vault,
                Err(error) => {
                    println!("Error: {}", error.to_string());
                    return Ok(());
                }
            };

            vault.run();
        }
    } else {
        println!("to many arguments provided, only provide encrypt, decrypt, vault or create");
    }

    Ok(())
}
