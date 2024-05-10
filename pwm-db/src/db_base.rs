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
