use pwm_db::{
    db_base::error::DatabaseError,
    db_encrypted::{forget_hash::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::{aes_wrapper::AesResult, zeroize::Zeroizing};

use crate::password::{password_confirmation, request_password};

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
            Ok(contents) => match AesResult::new(contents) {
                Ok(contents) => contents,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            },
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let password = match request_password("Enter master password") {
            Ok(value) => value,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new_deserialize_encrypted(&contents, password.as_bytes())?;

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
                    "help" | "h" => {
                        Self::help();
                    }
                    "insert" | "add" | "i" | "a" => {
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
                    "import" | "im" => {
                        if let Some(name) = itr.next() {
                            match self.import(name) {
                                Ok(()) => {}
                                Err(error) => {
                                    println!("Failed to import: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a file");
                        }
                    }
                    "export" | "ex" => {
                        if let Some(name) = itr.next() {
                            match self.export(name) {
                                Ok(()) => {}
                                Err(error) => {
                                    println!("Failed to export: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a file");
                        }
                    }
                    "remove" | "rm" => {
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
                    "replace" => {
                        if let Some(name) = itr.next() {
                            match self.replace(name, itr.next()) {
                                Ok(()) => (),
                                Err(error) => {
                                    println!("Failed to insert: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a key");
                        }
                    }
                    "rename" => {
                        if let (Some(name), Some(new_name)) = (itr.next(), itr.next()) {
                            match self.rename(name, new_name) {
                                Ok(()) => (),
                                Err(error) => {
                                    println!("Failed to rename: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected rename <name> <name>")
                        }
                    }
                    "get" | "g" => {
                        if let Some(data) = itr.next() {
                            match self.get(data) {
                                Ok(result) => {
                                    let pass = match String::from_utf8(result.as_slice().to_vec()) {
                                        Ok(val) => Zeroizing::new(val),
                                        Err(error) => {
                                            println!(
                                                "Failed to convert data to String: {}",
                                                error.to_string()
                                            );
                                            Zeroizing::new("".to_string())
                                        }
                                    };

                                    println!("{}", pass.as_str());
                                }
                                Err(error) => {
                                    println!("Failed to get: {}", error.to_string());
                                }
                            }
                        } else {
                            println!("Expected a key")
                        }
                    }
                    "list" | "ls" | "l" => match self.db.list() {
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
                    "save" | "s" => {
                        if let Some(value) = itr.next() {
                            self.serialize_and_save(value);
                        } else {
                            println!("Expected a filename");
                        }
                    }
                    "exit" | "quit" | "q" => {
                        break 'a;
                    }
                    _ => {
                        println!("Invalid command");
                    }
                }
            }
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

    fn import(&mut self, file: &str) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.insert_from_csv(file, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn export(&mut self, file: &str) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.export_to_csv(file, password.as_bytes())?;
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

    fn replace(&mut self, name: &str, new_data: Option<&str>) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let new_data_password: Zeroizing<String>;
        let new_data = if let Some(new_data) = new_data {
            new_data
        } else {
            match request_password("Enter new password") {
                Ok(password) => {
                    new_data_password = password;
                    new_data_password.as_str()
                }
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            }
        };
        self.db.replace(name, new_data.as_bytes(), password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn rename(&mut self, name: &str, new_name: &str) -> Result<(), DatabaseError> {
        let password = match request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.rename(name, new_name, password.as_bytes())?;
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

    fn serialize_and_save(&mut self, file: &str) {
        let password = match request_password("Enter master password") {
            Ok(pass) => pass,
            Err(error) => {
                println!("Error failed to get user input {}", error);
                return;
            }
        };

        let ciphertext = match self.db.serialize_encrypted(password.as_bytes()) {
            Ok(data) => data,
            Err(error) => {
                println!("Error failed to serialize database: {}", error);
                return;
            }
        };

        println!("Writing to file \"{}\"", file);
        match std::fs::write(file, ciphertext.as_ref()) {
            Ok(()) => (),
            Err(error) => {
                println!("Error failed to write to file: {}", error);
                return;
            }
        };
        self.changed = false;
    }

    fn help() {
        println!(
            "Vault:
    help                 - this menu 
    insert <key> <data?> - insert an element 
    import <file>        - import key/value pairs from csv
    export <file>        - export key/value pairs to csv
    remove <key>         - remove an element
    replace <key> <data?>- remove an element
    remove <name> <name> - rename an entry
    get    <key>         - retrieve an element
    save   <file>        - save to a file
    list                 - list all keys
    exit                 - exit the program"
        )
    }
}
