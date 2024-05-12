pub mod error;

use std::collections::btree_map::BTreeMap;

use serde::{Deserialize, Serialize};

use self::error::DatabaseError;

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

    pub fn rename(&mut self, name: &str, new_name: &str) -> Result<(), DatabaseError> {
        if let Some(data) = self.data.remove(name) {
            self.insert(new_name, data)?;
        } else {
            return Err(DatabaseError::NotFound)
        }
        Ok(())
    }

    pub fn replace(&mut self, name: &str, new_data: V) -> Result<(), DatabaseError> {
        if !self.data.contains_key(name) {
            return Err(DatabaseError::NotFound);
        }

        self.data.insert(name.to_string(), new_data);
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
    use crate::db_base::{error::DatabaseError, Database};

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

    #[test]
    fn db_insert_failed() {
        let mut db = Database::<i32>::new();
        db.insert("Hello", 5).unwrap();
        let result = db.insert("Hello", 5);
        assert!(result.unwrap_err() == DatabaseError::AlreadyExists)
    }

    #[test]
    fn db_rename() {
        let mut db = Database::<i32>::new();
        db.insert("Hello", 5).unwrap();
        db.rename("Hello", "Hello2").unwrap();
        if *db.get("Hello2").unwrap() != 5 {
            panic!("did not find 5");
        }
    }

    #[test]
    fn db_replace() {
        let mut db = Database::<i32>::new();
        db.insert("Hello", 5).unwrap();
        db.replace("Hello", 6).unwrap();
        if *db.get("Hello").unwrap() != 6 {
            panic!("did not find 6");
        }
    }
}
