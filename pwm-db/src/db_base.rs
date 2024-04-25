use std::collections::btree_map::BTreeMap;

use serde::{Deserialize, Serialize};

use pwm_lib::zeroize::Zeroize;

#[derive(Debug)]
pub enum DatabaseError {
    NotFound,
    AlreadyExists,
    FailedHash,
    FailedAes,
    LockError,
    InvalidPassword,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::NotFound => f.write_str("Not found"),
            Self::AlreadyExists => f.write_str("Already exists"),
            Self::FailedHash => f.write_str("Failed hash"),
            Self::FailedAes => f.write_str("Failed aes encryption"),
            Self::LockError => f.write_str("Failed to get mutex lock on db"),
            Self::InvalidPassword => f.write_str("Invalid password provided"),
        };
    }
}

impl std::error::Error for DatabaseError {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Database<V>
where
    V: Zeroize,
{
    data: BTreeMap<String, V>,
}

impl<V> Database<V>
where
    V: Zeroize,
{
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
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
