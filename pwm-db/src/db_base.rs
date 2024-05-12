pub mod error;

use std::collections::btree_map::BTreeMap;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};

use self::error::DatabaseError;

#[derive(Serialize, Deserialize, Debug)]
pub struct Database<V> 
{
    data: BTreeMap<String, V>,
    prev_list_changed: bool,
    prev_list: Vec<String>,
    prev_pattern: String,
}

impl<V> Database<V> {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            prev_list_changed: true,
            prev_list: Vec::new(),
            prev_pattern: String::new(),
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
        self.prev_list_changed = true;
        Ok(())
    }

    pub fn rename(&mut self, name: &str, new_name: &str) -> Result<(), DatabaseError> {
        if let Some(data) = self.data.remove(name) {
            self.insert(new_name, data)?;
        } else {
            return Err(DatabaseError::NotFound)
        }

        self.prev_list_changed = true;
        Ok(())
    }

    pub fn replace(&mut self, name: &str, new_data: V) -> Result<(), DatabaseError> {
        if !self.data.contains_key(name) {
            return Err(DatabaseError::NotFound);
        }

        self.prev_list_changed = true;
        self.data.insert(name.to_string(), new_data);
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), DatabaseError> {
        if !self.data.contains_key(name) {
            return Err(DatabaseError::NotFound);
        }

        self.prev_list_changed = true;
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

    pub fn list_fuzzy_match(&mut self, pattern: &str) -> Result<&Vec<String>, DatabaseError> {
        if !self.prev_list_changed && self.prev_pattern == pattern {
            return Ok(&self.prev_list);
        } else {
            let list = self.list()?;
            self.prev_list = list;
            self.prev_pattern = String::from(pattern);
            self.prev_list_changed = false;
        }

        if pattern.is_empty() {
            return Ok(&self.prev_list);
        }

        let matcher = SkimMatcherV2::default();

        let mut rated_list = Vec::new();

        for element in self.prev_list.iter() {
            let score = match matcher.fuzzy_match(element.as_str(), pattern) {
                Some(score) => score,
                None => 0,
            };
            rated_list.push((score, element))
        }

        rated_list.sort_by(|a, b| b.0.cmp(&a.0));

        self.prev_list = rated_list
            .iter()
            .filter(|x| x.0 != 0)
            .map(|x| x.1.to_string())
            .collect::<Vec<String>>();

        Ok(&self.prev_list)
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

    #[test]
    fn db_fuzzy_find() {
        let mut db = Database::<i32>::new();
        db.insert("Hello", 5).unwrap();
        db.insert("Hello2", 5).unwrap();
        db.insert("Hello3", 5).unwrap();
        db.insert("Hello4", 5).unwrap();

        let list = db.list_fuzzy_match("4").unwrap();
        assert!(list.len() == 1);
        assert_eq!(list.contains(&"Hello4".to_string()), true);

        let list = db.list_fuzzy_match("2").unwrap();
        assert!(list.len() == 1);
        assert_eq!(list.contains(&"Hello2".to_string()), true);
    }
}
