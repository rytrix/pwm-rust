use pwm_db::{
    db_base::error::DatabaseError,
    db_encrypted::{db_interface::DatabaseInterface, DatabaseEncrypted},
};
use pwm_lib::{encryption::EncryptionResult, random::random_password, zeroize::Zeroizing};

use crate::parser::Parser;

pub struct Vault<I, O>
where
    I: std::io::BufRead,
    O: std::io::Write,
{
    db: DatabaseEncrypted,
    changed: bool,
    reader: I,
    writer: O,
    test_mode: bool,
    clipboard: arboard::Clipboard,
}

impl<I, O> Vault<I, O>
where
    I: std::io::BufRead,
    O: std::io::Write,
{
    fn new_internal<In, Out>(
        mut reader: In,
        writer: Out,
        test_mode: bool,
    ) -> Result<Vault<In, Out>, DatabaseError>
    where
        In: std::io::BufRead,
        Out: std::io::Write,
    {
        let password = if test_mode {
            match crate::password::password_confirmation_test(&mut reader) {
                Ok(password) => password,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            }
        } else {
            match crate::password::password_confirmation() {
                Ok(password) => password,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            }
        };

        let db = DatabaseEncrypted::new(password.as_bytes())?;

        let clipboard = match arboard::Clipboard::new() {
            Ok(clipboard) => clipboard,
            Err(error) => return Err(DatabaseError::ClipboardError(error.to_string())),
        };

        Ok(Vault {
            db,
            changed: true,
            reader,
            writer,
            test_mode,
            clipboard,
        })
    }

    pub fn new() -> Result<Vault<std::io::BufReader<std::io::Stdin>, std::io::Stdout>, DatabaseError>
    {
        let reader = std::io::BufReader::new(std::io::stdin());
        let writer = std::io::stdout();
        Self::new_internal(reader, writer, false)
    }

    fn new_from_file_internal<In, Out>(
        file: &str,
        mut reader: In,
        writer: Out,
        test_mode: bool,
    ) -> Result<Vault<In, Out>, DatabaseError>
    where
        In: std::io::BufRead,
        Out: std::io::Write,
    {
        let contents = match std::fs::read(file) {
            Ok(contents) => match EncryptionResult::new(contents) {
                Ok(contents) => contents,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            },
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let password = if test_mode {
            match crate::password::request_password_test(&mut reader, "Enter master password") {
                Ok(password) => password,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            }
        } else {
            match crate::password::request_password("Enter master password") {
                Ok(value) => value,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            }
        };

        let db = DatabaseEncrypted::new_deserialize_encrypted(&contents, password.as_bytes())?;

        let clipboard = match arboard::Clipboard::new() {
            Ok(clipboard) => clipboard,
            Err(error) => return Err(DatabaseError::ClipboardError(error.to_string())),
        };

        Ok(Vault {
            db,
            changed: false,
            reader,
            writer,
            test_mode,
            clipboard,
        })
    }

    pub fn new_from_file(
        file: &str,
    ) -> Result<Vault<std::io::BufReader<std::io::Stdin>, std::io::Stdout>, DatabaseError> {
        let reader = std::io::BufReader::new(std::io::stdin());
        let writer = std::io::stdout();
        Self::new_from_file_internal(file, reader, writer, false)
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        self.help()?;
        Ok(self.run_without_help()?)
    }

    fn run_without_help(&mut self) -> std::io::Result<()> {
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
                                writeln!(self.writer, "{}", error.to_string())?;
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

                                if self.test_mode {
                                    writeln!(self.writer, "{}", pass.as_str())?;
                                } else {
                                    match self.clipboard.set_text(pass.as_str()) {
                                        Ok(()) => {
                                            writeln!(self.writer, "copied to clipboard")?;
                                        }
                                        Err(error) => {
                                            writeln!(
                                                self.writer,
                                                "failed to copy to clipboard: {}",
                                                error.to_string()
                                            )?;
                                        }
                                    };
                                }
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
        let password = match self.request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let data_not_entered: Zeroizing<String>;

        let data = match data {
            Some(data) => data.as_bytes(),
            None => {
                data_not_entered =
                    match self.request_password("Data wasn't provided, provide it here") {
                        Ok(password) => password,
                        Err(error) => return Err(DatabaseError::InputError(error.to_string())),
                    };
                data_not_entered.as_bytes()
            }
        };

        self.db.insert(name, data, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn import(&mut self, file: &str) -> Result<(), DatabaseError> {
        let password = match self.request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.insert_from_csv(file, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn export(&mut self, file: &str) -> Result<(), DatabaseError> {
        let password = match self.request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.export_to_csv(file, password.as_bytes())?;

        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        let password = match self.request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.remove(name, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn replace(&mut self, name: &str, new_data: Option<&str>) -> Result<(), DatabaseError> {
        let password = match self.request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let new_data_password: Zeroizing<String>;
        let new_data = if let Some(new_data) = new_data {
            new_data
        } else {
            match self.request_password("Enter new password") {
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
        let password = match self.request_password("Enter the master password") {
            Ok(password) => password,
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };
        self.db.rename(name, new_name, password.as_bytes())?;
        self.changed = true;

        Ok(())
    }

    fn get(&mut self, name: &str) -> Result<EncryptionResult, DatabaseError> {
        let password = match self.request_password("Enter the master password") {
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
        let password = match self.request_password("Enter master password") {
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
    remove  <key>         - remove an element
    replace <key> <data?> - remove an element
    rename  <name> <name> - rename an entry
    get     <key>         - retrieve an element
    save    <file>        - save to a file
    list    <pattern?>    - list all keys
    search  <pattern?>    - search all keys
    import  <file>        - import key/value pairs from csv
    export  <file>        - export key/value pairs to csv
    pw      <length>      - generate a password
    exit                  - exit the program"
        )?;
        Ok(())
    }

    fn request_password(&mut self, prompt: &str) -> std::io::Result<Zeroizing<String>> {
        if self.test_mode {
            crate::password::request_password_test(&mut self.reader, prompt)
        } else {
            crate::password::request_password(prompt)
        }
    }

    #[allow(dead_code)]
    fn password_confirmation(&mut self) -> std::io::Result<Zeroizing<String>> {
        if self.test_mode {
            crate::password::password_confirmation_test(&mut self.reader)
        } else {
            crate::password::password_confirmation()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Vault;
    use std::io::{BufRead, BufReader, Cursor, Write};

    fn new_vault(text: &str) -> Vault<BufReader<Cursor<&[u8]>>, Cursor<Vec<u8>>> {
        let input = BufReader::new(Cursor::new(text.as_bytes()));
        let output = Cursor::new(Vec::<u8>::new());

        let vault =
            Vault::<BufReader<Cursor<&[u8]>>, Cursor<Vec<u8>>>::new_internal(input, output, true)
                .unwrap();

        vault
    }

    fn new_vault_from_file<'a, 'b>(
        file: &'a str,
        text: &'b str,
    ) -> Vault<BufReader<Cursor<&'b [u8]>>, Cursor<Vec<u8>>> {
        let input = BufReader::new(Cursor::new(text.as_bytes()));
        let output = Cursor::new(Vec::<u8>::new());

        let vault = Vault::<BufReader<Cursor<&[u8]>>, Cursor<Vec<u8>>>::new_from_file_internal(
            file, input, output, true,
        )
        .unwrap();

        vault
    }

    fn reset_cursors(
        vault: &mut Vault<BufReader<Cursor<&[u8]>>, Cursor<Vec<u8>>>,
        input: &'static str,
    ) {
        let input = BufReader::new(Cursor::new(input.as_bytes()));
        let output = Cursor::new(Vec::<u8>::new());
        vault.reader = input;
        vault.writer = output;
    }

    fn run_command(
        vault: &mut Vault<BufReader<Cursor<&[u8]>>, Cursor<Vec<u8>>>,
    ) -> std::io::Result<bool> {
        let mut input = String::new();
        match vault.reader.read_line(&mut input) {
            Ok(count) => count,
            Err(error) => {
                writeln!(vault.writer, "User input error: {}", error.to_string())?;
                0
            }
        };

        if vault.handle_input(&mut input)? {
            return Ok(true);
        }

        Ok(false)
    }

    fn output_to_string(vault: &mut Vault<BufReader<Cursor<&[u8]>>, Cursor<Vec<u8>>>) -> String {
        let bytes = vault.writer.get_ref();
        let string = String::from_utf8(bytes.to_vec()).unwrap();
        return string;
    }

    #[test]
    fn test_insert_get() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(&mut vault, "insert test 123\n12\nget test\n12\n");

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);

        assert_eq!(string, "123\n");
    }

    #[test]
    fn test_insert_get_extended_name() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(
            &mut vault,
            "insert \"test 123\" 123\n12\nget \"test 123\"\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);

        assert_eq!(string, "123\n");
    }

    #[test]
    fn test_replace() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(
            &mut vault,
            "insert test 123\n12\nreplace test 1234\n12\nget test\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);

        assert_eq!(string, "1234\n");
    }

    #[test]
    fn test_rename() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(
            &mut vault,
            "insert test 123\n12\nrename test test2\n12\nget test2\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);

        assert_eq!(string, "123\n");
    }

    #[test]
    fn test_list_search() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(
            &mut vault,
            "insert user1 123\n12\ninsert user2 123\n12\ninsert user3 123\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "");

        reset_cursors(&mut vault, "list\n");
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "user1, user2, user3\n");

        reset_cursors(&mut vault, "search\n");
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "user1, user2, user3\n");

        reset_cursors(&mut vault, "list 1\n");
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "user1\n");

        reset_cursors(&mut vault, "search 1\n");
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "user1\n");
    }

    #[test]
    fn test_import() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(&mut vault, "import tests/users.csv\n12\nget user1\n12\n");

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);

        assert_eq!(string, "password1\n");
    }

    #[test]
    fn test_import_export() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(
            &mut vault,
            "import tests/users.csv\n12\nexport tests/users_test.csv\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let imported = std::fs::read("tests/users.csv").unwrap();
        let exported = std::fs::read("tests/users_test.csv").unwrap();
        std::fs::remove_file("tests/users_test.csv").unwrap();

        assert_eq!(imported, exported);
    }

    #[test]
    fn test_vault_save_load() {
        let mut vault = new_vault("12\n12\n");
        reset_cursors(
            &mut vault,
            "import tests/users.csv\n12\nsave tests/Vault\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let mut vault = new_vault_from_file("tests/Vault", "12\n");
        reset_cursors(
            &mut vault,
            "get user0\n12\nexport tests/save_load_test_users.csv\n12\n",
        );

        run_command(&mut vault).unwrap();
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "password0\n");

        let imported = std::fs::read("tests/users.csv").unwrap();
        let exported = std::fs::read("tests/save_load_test_users.csv").unwrap();
        std::fs::remove_file("tests/save_load_test_users.csv").unwrap();
        std::fs::remove_file("tests/Vault").unwrap();

        assert_eq!(imported, exported);

        reset_cursors(
            &mut vault,
            "q",
        );
        run_command(&mut vault).unwrap();

        let string = output_to_string(&mut vault);
        assert_eq!(string, "");
    }
}
