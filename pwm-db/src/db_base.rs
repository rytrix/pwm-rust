use std::collections::btree_map::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum DatabaseError {
    NotFound,
    AlreadyExists,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::NotFound => {
                f.write_str("Not found")
            }
            Self::AlreadyExists => {
                f.write_str("Already exists")
            }
        }
    }
}

impl std::error::Error for DatabaseError {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Database<V> {
    data: BTreeMap<String, V>,
}

impl<V> Database<V> {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, value: V) -> Result<(), DatabaseError> {
        if self.data.contains_key(&name) {
            return Err(DatabaseError::AlreadyExists);
        }

        self.data.insert(name, value);
        Ok(())
    }

    pub fn remove(&mut self, name: String) -> Result<(), DatabaseError> {
        if !self.data.contains_key(&name) {
            return Err(DatabaseError::NotFound);
        }

        self.data.remove(&name);
        Ok(())
    }

    pub fn get(&self, name: String) -> Result<&V, DatabaseError> {
        let value = self.data.get(&name);
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
        db.insert("Hello".to_string(), 5).unwrap();
        db.insert("Hello2".to_string(), 5).unwrap();

        let val = db.get("Hello".to_string()).unwrap();
        if *val != 5 {
            panic!("did not find 5");
        }
        db.remove("Hello".to_string()).unwrap();
    }
}