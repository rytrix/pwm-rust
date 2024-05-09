use std::collections::btree_map::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum DatabaseError {
    NotFound,
    AlreadyExists,
    FailedHash(String),
    FailedAes(String),
    LockError,
    InvalidPassword,
    InputError(String),
    OutputError(String),
    FailedSerialize,
    FailedDeserialize,
    CsvError(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::NotFound => f.write_str("Not found"),
            Self::AlreadyExists => f.write_str("Already exists"),
            Self::FailedHash(msg) => f.write_fmt(std::format_args!("Failed hash: {}", msg)),
            Self::FailedAes(msg) => f.write_fmt(std::format_args!("{}", msg)),
            Self::LockError => f.write_str("Failed to get mutex lock on db"),
            Self::InvalidPassword => f.write_str("Invalid password provided"),
            Self::InputError(msg) => f.write_fmt(std::format_args!("Input error: {}", msg)),
            Self::OutputError(msg) => f.write_fmt(std::format_args!("Output error: {}", msg)),
            Self::FailedSerialize => f.write_str("Failed to serialize"),
            Self::FailedDeserialize => f.write_str("Failed to deserialize"),
            Self::CsvError(msg) => f.write_fmt(std::format_args!("Csv error: {}", msg)),
        };
    }
}

impl std::error::Error for DatabaseError {}

impl From<csv::Error> for DatabaseError {
    fn from(value: csv::Error) -> Self {
        Self::CsvError(value.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Database<V> 
{
    data: BTreeMap<String, V>,
}

impl<V> Database<V> {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn as_ref(&self) -> &Self {
        &self
    }

    pub fn as_ref_mut(&mut self) -> &mut Self {
        self
    }

    pub fn insert(&mut self, name: &str, value: V) -> Result<(), DatabaseError> {
        if self.data.contains_key(name) {
            return Err(DatabaseError::AlreadyExists);
        }

        self.data.insert(name.to_string(), value);
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        if !self.data.contains_key(name) {
            return Err(DatabaseError::NotFound);
        }

        self.data.remove(name);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<&V, DatabaseError> {
        let value = self.data.get(name);
        return match value {
            Some(value) => Ok(value),
            None => Err(DatabaseError::NotFound),
        };
    }

    pub fn list(&self) -> Result<Vec<String>, DatabaseError> {
        let keys = self.data.iter().map(|(key, _)| key.clone()).collect();
        Ok(keys)
    }
}

#[cfg(test)]
mod test {
    use crate::db_base::Database;

    #[test]
    fn db_insert() {
        let mut db = Database::<i32>::new();
        db.insert("Hello", 5).unwrap();
        db.insert("Hello2", 5).unwrap();

        let val = db.get("Hello").unwrap();
        if *val != 5 {
            panic!("did not find 5");
        }
        db.remove("Hello").unwrap();
    }
}
