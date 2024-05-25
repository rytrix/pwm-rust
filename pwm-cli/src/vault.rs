use pwm_db::{
    db_base::error::DatabaseError,
    db_encrypted::{db_interface::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::{encryption::EncryptionResult, random::random_password, zeroize::Zeroizing};

use crate::parser::Parser;
use crate::password::{password_confirmation, request_password};

pub struct Vault<I, O>
where
    I: std::io::BufRead,
    O: std::io::Write,
{
    db: DatabaseEncrypted,
    changed: bool,
    reader: I,
    writer: O,
}

impl<I, O> Vault<I, O>
where
    I: std::io::BufRead,
    O: std::io::Write,
{
    pub fn new() -> Result<Vault<std::io::BufReader<std::io::Stdin>, std::io::Stdout>, DatabaseError>
    {
        let mut reader = std::io::BufReader::new(std::io::stdin());
        let password = match password_confirmation(&mut reader) {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new(password.as_bytes())?;
        Ok(Vault {
            db,
            changed: true,
            reader,
            writer: std::io::stdout(),
        })
    }

    pub fn new_from_file(
        file: &str,
    ) -> Result<Vault<std::io::BufReader<std::io::Stdin>, std::io::Stdout>, DatabaseError> {
        let mut reader = std::io::BufReader::new(std::io::stdin());
        let contents = match std::fs::read(file) {
            Ok(contents) => match EncryptionResult::new(contents) {
                Ok(contents) => contents,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            },
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let password = match request_password(&mut reader, "Enter master password") {
            Ok(value) => value,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new_deserialize_encrypted(&contents, password.as_bytes())?;

        Ok(Vault {
            db,
            changed: false,
            reader,
            writer: std::io::stdout(),
        })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        self.help()?;

        let mut input = String::new();
        'a: loop {
            input.clear();
            match self.reader.read_line(&mut input) {
                Ok(count) => count,
                Err(error) => {
                    writeln!(self.writer, "User input error: {}", error.to_string())?;
                    0
                }
            };

            if self.handle_input(&mut input)? {
                break 'a;
            }
        }

        if self.changed {
            self.handle_changed(&mut input)?;
        }

        Ok(())
    }

    fn handle_input(&mut self, input: &mut String) -> std::io::Result<bool> {
        let parser = Parser::new(input.as_str());
        let mut itr = parser.iter();

        if let Some(value) = itr.next() {
            match value {
                "help" | "h" => {
                    self.help()?;
                }
                "insert" | "add" | "i" | "a" => {
                    if let Some(name) = itr.next() {
                        match self.insert(name, itr.next()) {
                            Ok(()) => (),
                            Err(error) => {
                                writeln!(self.writer, "Failed to insert: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected a key")?;
                    }
                }
                "import" | "im" => {
                    if let Some(name) = itr.next() {
                        match self.import(name) {
                            Ok(()) => {}
                            Err(error) => {
                                writeln!(self.writer, "Failed to import: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected a file")?;
                    }
                }
                "export" | "ex" => {
                    if let Some(name) = itr.next() {
                        match self.export(name) {
                            Ok(()) => {}
                            Err(error) => {
                                writeln!(self.writer, "Failed to export: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected a file")?;
                    }
                }
                "remove" | "rm" => {
                    if let Some(data) = itr.next() {
                        match self.remove(data) {
                            Ok(()) => (),
                            Err(error) => {
                                writeln!(self.writer, "Failed to remove: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected a key")?;
                    }
                }
                "replace" => {
                    if let Some(name) = itr.next() {
                        match self.replace(name, itr.next()) {
                            Ok(()) => (),
                            Err(error) => {
                                writeln!(self.writer, "Failed to insert: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected a key")?;
                    }
                }
                "rename" => {
                    if let (Some(name), Some(new_name)) = (itr.next(), itr.next()) {
                        match self.rename(name, new_name) {
                            Ok(()) => (),
                            Err(error) => {
                                writeln!(self.writer, "Failed to rename: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected rename <name> <name>")?;
                    }
                }
                "get" | "g" => {
                    if let Some(data) = itr.next() {
                        match self.get(data) {
                            Ok(result) => {
                                let pass = match String::from_utf8(result.as_slice().to_vec()) {
                                    Ok(val) => Zeroizing::new(val),
                                    Err(error) => {
                                        writeln!(
                                            self.writer,
                                            "Failed to convert data to String: {}",
                                            error.to_string()
                                        )?;
                                        Zeroizing::new("".to_string())
                                    }
                                };

                                writeln!(self.writer, "{}", pass.as_str())?;
                            }
                            Err(error) => {
                                writeln!(self.writer, "Failed to get: {}", error.to_string())?;
                            }
                        }
                    } else {
                        writeln!(self.writer, "Expected a key")?;
                    }
                }
                "list" | "ls" => {
                    match self.list(itr.next()) {
                        Ok(()) => (),
                        Err(error) => {
                            writeln!(self.writer, "Failed to list: {}", error.to_string())?;
                        }
                    };
                }
                "search" => {
                    match self.list(itr.next()) {
                        Ok(()) => (),
                        Err(error) => {
                            writeln!(self.writer, "Failed to search: {}", error.to_string())?;
                        }
                    };
                }
                "save" | "s" => {
                    if let Some(value) = itr.next() {
                        self.serialize_and_save(value)?;
                    } else {
                        writeln!(self.writer, "Expected a filename")?;
                    }
                }
                "pw" => {
                    if let Some(value) = itr.next() {
                        self.generate_password(value)?;
                    } else {
                        writeln!(self.writer, "Expected a length")?;
                    }
                }
                "exit" | "quit" | "q" => {
                    return Ok(true);
                }
                _ => {
                    writeln!(self.writer, "Invalid command")?;
                }
            }
        }

        Ok(false)
    }

    fn handle_changed(&mut self, input: &mut String) -> std::io::Result<()> {
        'a: loop {
            writeln!(
                self.writer,
                "Vault changed, do you want to save the file? (Y, N)"
            )?;
            input.clear();
            let _ = match self.reader.read_line(input) {
                Ok(count) => count,
                Err(error) => {
                    writeln!(self.writer, "User input error: {}", error.to_string())?;
                    0
                }
            };

            let mut itr = input.split_whitespace();
            if let Some(value) = itr.next() {
                match value.to_ascii_lowercase().as_str() {
                    "y" | "yes" => {
                        writeln!(self.writer, "Enter file name")?;
                        input.clear();
                        let _ = match self.reader.read_line(input) {
                            Ok(count) => count,
                            Err(error) => {
                                writeln!(self.writer, "User input error: {}", error.to_string())?;
                                0
                            }
                        };

                        let mut itr = input.split_whitespace();
                        if let Some(value) = itr.next() {
                            self.serialize_and_save(value)?;
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

        Ok(())
    }

    fn insert(&mut self, name: &str, data: Option<&str>) -> Result<(), DatabaseError> {
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let data_not_entered: Zeroizing<String>;

        self.db.insert(
            name,
            match data {
                Some(data) => data.as_bytes(),
                None => {
                    data_not_entered = match request_password(
                        &mut self.reader,
                        "Data wasn't provided, provide it here",
                    ) {
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
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.insert_from_csv(file, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn export(&mut self, file: &str) -> Result<(), DatabaseError> {
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.export_to_csv(file, password.as_bytes())?;

        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.remove(name, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn replace(&mut self, name: &str, new_data: Option<&str>) -> Result<(), DatabaseError> {
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let new_data_password: Zeroizing<String>;
        let new_data = if let Some(new_data) = new_data {
            new_data
        } else {
            match request_password(&mut self.reader, "Enter new password") {
                Ok(password) => {
                    new_data_password = password;
                    new_data_password.as_str()
                }
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            }
        };
        self.db
            .replace(name, new_data.as_bytes(), password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn rename(&mut self, name: &str, new_name: &str) -> Result<(), DatabaseError> {
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.rename(name, new_name, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn get(&mut self, name: &str) -> Result<EncryptionResult, DatabaseError> {
        let password = match request_password(&mut self.reader, "Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.get(name, password.as_bytes())
    }

    fn list(&mut self, pattern: Option<&str>) -> Result<(), DatabaseError> {
        let list_lifetime: Vec<String>;

        let list = if let Some(pattern) = pattern {
            self.db.list_fuzzy_match(pattern)?
        } else {
            list_lifetime = self.db.list()?;
            &list_lifetime
        };

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
        writeln!(self.writer, "{}", list_string)?;

        Ok(())
    }

    fn serialize_and_save(&mut self, file: &str) -> std::io::Result<()> {
        let password = match request_password(&mut self.reader, "Enter master password") {
            Ok(pass) => pass,
            Err(error) => {
                writeln!(self.writer, "Error failed to get user input {}", error)?;
                return Ok(());
            }
        };

        let ciphertext = match self.db.serialize_encrypted(password.as_bytes()) {
            Ok(data) => data,
            Err(error) => {
                writeln!(self.writer, "Error failed to serialize database: {}", error)?;
                return Ok(());
            }
        };

        writeln!(self.writer, "Writing to file \"{}\"", file)?;
        match std::fs::write(file, ciphertext.as_ref()) {
            Ok(()) => (),
            Err(error) => {
                writeln!(self.writer, "Error failed to write to file: {}", error)?;
                return Ok(());
            }
        };
        self.changed = false;

        Ok(())
    }

    fn generate_password(&mut self, length: &str) -> std::io::Result<()> {
        let length = match length.parse::<usize>() {
            Ok(length) => length,
            Err(_error) => {
                writeln!(self.writer, "Invalid length input")?;
                return Ok(());
            }
        };
        let password = match random_password(length) {
            Ok(password) => password,
            Err(error) => {
                writeln!(self.writer, "Failed to generate password: {}", error)?;
                return Ok(());
            }
        };
        writeln!(self.writer, "Generated: \"{}\"", password)?;
        Ok(())
    }

    fn help(&mut self) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "Vault:
    help                  - this menu 
    insert  <key> <data?> - insert an element 
    import  <file>        - import key/value pairs from csv
    export  <file>        - export key/value pairs to csv
    remove  <key>         - remove an element
    replace <key> <data?> - remove an element
    remove  <name> <name> - rename an entry
    get     <key>         - retrieve an element
    save    <file>        - save to a file
    list    <pattern?>    - list all keys
    search  <pattern?>    - search all keys
    pw      <length>      - generate a password
    exit                  - exit the program"
        )?;
        Ok(())
    }
}
