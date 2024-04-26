use pwm_db::{db_base::DatabaseError, db_encrypted::DatabaseEncryptedAsync};
use pwm_lib::{aes_wrapper::AesResult, zeroize::Zeroizing};
use std::io::Write;

use crate::crypt_file::{password_confirmation, request_password};

pub struct Vault {
    db: DatabaseEncryptedAsync,
    changed: bool,
}

impl Vault {
    pub async fn new() -> Result<Self, DatabaseError> {
        let password = match password_confirmation() {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncryptedAsync::new(password.as_bytes()).await?;
        Ok(Self { db, changed: true })
    }

    pub async fn new_from_file(file: &str) -> Result<Self, DatabaseError> {
        let contents = match std::fs::read(file) {
            Ok(contents) => Zeroizing::new(contents),
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncryptedAsync::new_deserialize(contents.as_slice()).await?;

        Ok(Self { db, changed: false })
    }

    pub async fn run(&mut self) {
        Self::help();

        let mut input = String::new();
        'a: loop {
            input.clear();
            match std::io::stdin().read_line(&mut input) {
                Ok(count) => count,
                Err(error) => {
                    println!("User input error: {}", error.to_string());
                    0
                }
            };

            let mut itr = input.split_whitespace();
            if let Some(value) = itr.next() {
                match value {
                    "help" => {
                        Self::help();
                    }
                    "insert" => {
                        if let Some(name) = itr.next() {
                            if let Some(data) = itr.next() {
                                match self.insert(name, Some(data)).await {
                                    Ok(()) => (),
                                    Err(error) => {
                                        println!("Failed to insert: {}", error.to_string());
                                    }
                                }
                            } else {
                                match self.insert(name, None).await {
                                    Ok(()) => (),
                                    Err(error) => {
                                        println!("Failed to insert: {}", error.to_string());
                                    }
                                }
                            }
                        } else {
                            println!("Expected a key");
                        }
                    }
                    "remove" => {
                        if let Some(data) = itr.next() {
                            match self.remove(data).await {
                                Ok(()) => (),
                                Err(error) => {
                                    println!("Failed to remove: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a key")
                        }
                    }
                    "get" => {
                        if let Some(data) = itr.next() {
                            match self.get(data).await {
                                Ok(result) => {
                                    println!(
                                        "insecure test \"{}\"",
                                        String::from_utf8(result.as_slice().to_vec())
                                            .expect("failed to convert crypt result to string")
                                    )
                                }
                                Err(error) => {
                                    println!("Failed to remove: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a key")
                        }
                    }
                    "save" => {
                        if let Some(value) = itr.next() {
                            self.serialize_and_save(value).await;
                            self.changed = false;
                        } else {
                            println!("Expected a filename");
                        }
                    }
                    "exit" | "quit" | "q" => {
                        break 'a;
                    }
                    _ => {}
                }
            }

            println!("Operation complete\n");
        }

        if self.changed {
            'a: loop {
                println!("Vault changed, do you want to save the file? (Y, N)");
                input.clear();
                let _ = match std::io::stdin().read_line(&mut input) {
                    Ok(count) => count,
                    Err(error) => {
                        println!("User input error: {}", error.to_string());
                        0
                    }
                };

                let mut itr = input.split_whitespace();
                if let Some(value) = itr.next() {
                    match value.to_ascii_lowercase().as_str() {
                        "y" | "yes" => {
                            println!("Enter file name");
                            input.clear();
                            let _ = match std::io::stdin().read_line(&mut input) {
                                Ok(count) => count,
                                Err(error) => {
                                    println!("User input error: {}", error.to_string());
                                    0
                                }
                            };

                            let mut itr = input.split_whitespace();
                            if let Some(value) = itr.next() {
                                self.serialize_and_save(value).await;
                            }
                            break 'a;
                        }
                        "n" | "no" => {
                            break 'a;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn insert(&mut self, name: &str, data: Option<&str>) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let data_not_entered: Zeroizing<String>;

        self.db
            .insert(
                name,
                match data {
                    Some(data) => data.as_bytes(),
                    None => {
                        data_not_entered =
                            match request_password("Data wasn't provided, provide it here") {
                                Ok(password) => password,
                                Err(error) => {
                                    return Err(DatabaseError::InputError(error.to_string()))
                                }
                            };
                        data_not_entered.as_bytes()
                    }
                },
                password.as_bytes(),
            )
            .await?;

        self.changed = true;
        Ok(())
    }

    async fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        self.db.remove(name, password.as_bytes()).await?;

        self.changed = true;

        Ok(())
    }

    async fn get(&self, name: &str) -> Result<AesResult, DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        self.db.get(name, password.as_bytes()).await
    }

    async fn serialize_and_save(&self, file: &str) {
        let data = match self.db.serialize().await {
            Ok(data) => data,
            Err(error) => {
                println!("Error failed to serialize database: {}", error);
                return;
            }
        };

        println!("Writing to file \"{}\"", file);
        match std::fs::write(file, &data) {
            Ok(()) => (),
            Err(error) => {
                println!("Error failed to write to file: {}", error);
                return;
            }
        };
    }

    fn help() {
        println!(
            "Vault:
    help                - this menu 
    insert <key> <data> - insert an element 
    remove <key>        - remove an element
    get    <key>        - retrieve an element
    save   <file>       - save to a file
    exit                - exit the program"
        )
    }
}
