use pwm_db::{db_base::DatabaseError, db_encrypted::DatabaseEncrypted};
use pwm_lib::{aes_wrapper::AesResult, zeroize::Zeroizing};

use crate::crypt_file::{password_confirmation, request_password};

// enum VaultResult {
//     None(Result<(), DatabaseError>),
//     Aes(Result<AesResult, DatabaseError>),
// }

pub struct Vault {
    db: DatabaseEncrypted,
    changed: bool,
}

impl Vault {
    pub fn new() -> Result<Self, DatabaseError> {
        let password = match password_confirmation() {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new(password.as_bytes())?;
        Ok(Self { db, changed: true })
    }

    pub fn new_from_file(file: &str) -> Result<Self, DatabaseError> {
        let contents = match std::fs::read(file) {
            Ok(contents) => Zeroizing::new(contents),
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new_deserialize(contents.as_slice())?;

        Ok(Self { db, changed: false })
    }

    pub fn run(&mut self) {
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
                            match self.insert(name, itr.next()) {
                                Ok(()) => (),
                                Err(error) => {
                                    println!("Failed to insert: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a key");
                        }
                    }
                    "remove" => {
                        if let Some(data) = itr.next() {
                            match self.remove(data) {
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
                            match self.get(data) {
                                Ok(result) => {
                                    println!(
                                        "insecure test \"{}\"",
                                        String::from_utf8(result.as_slice().to_vec())
                                            .expect("failed to convert crypt result to string")
                                    )
                                }
                                Err(error) => {
                                    println!("Failed to get: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a key")
                        }
                    }
                    "list" | "ls" => match self.db.list() {
                        Ok(list) => {
                            let mut list_string = String::new();
                            let last_value = list.len() - 1;
                            let mut itr_number = 0;
                            for value in list {
                                if itr_number != last_value {
                                    list_string += format!("{}, ", value).as_str();
                                } else {
                                    list_string += format!("{}", value).as_str();
                                }
                                itr_number += 1;
                            }
                            println!("{}", list_string);
                        }
                        Err(error) => {
                            println!("Failed to list: {}", error.to_string());
                        }
                    },
                    "save" => {
                        if let Some(value) = itr.next() {
                            self.serialize_and_save(value);
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

            // println!("Operation complete\n");
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
                                self.serialize_and_save(value);
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

    fn insert(&mut self, name: &str, data: Option<&str>) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let data_not_entered: Zeroizing<String>;

        self.db.insert(
            name,
            match data {
                Some(data) => data.as_bytes(),
                None => {
                    data_not_entered =
                        match request_password("Data wasn't provided, provide it here") {
                            Ok(password) => password,
                            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
                        };
                    data_not_entered.as_bytes()
                }
            },
            password.as_bytes(),
        )?;

        self.changed = true;
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        self.db.remove(name, password.as_bytes())?;

        self.changed = true;

        Ok(())
    }

    fn get(&self, name: &str) -> Result<AesResult, DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        self.db.get(name, password.as_bytes())
    }

    fn serialize_and_save(&self, file: &str) {
        // TODO encrypt on save
        let data = match self.db.serialize() {
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
    list                - list all keys
    exit                - exit the program"
        )
    }
}
