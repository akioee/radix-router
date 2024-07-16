use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
pub enum MetaType {
    Str(String),
    Num(usize),
    Bol(bool),
}

impl From<&str> for MetaType {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl From<usize> for MetaType {
    fn from(value: usize) -> Self {
        Self::Num(value)
    }
}

impl From<bool> for MetaType {
    fn from(value: bool) -> Self {
        Self::Bol(value)
    }
}

#[derive(Debug, Default)]
pub struct Meta(HashMap<String, MetaType>);

impl Deref for Meta {
    type Target = HashMap<String, MetaType>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Meta {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Meta {
    pub fn merge_rhs(&mut self, meta: &Meta) {
        self.extend(meta.iter().map(|(k, v)| (k.clone(), v.clone())))
    }

    pub fn insert(&mut self, key: &str, value: MetaType) -> Option<MetaType> {
        self.0.insert(key.to_owned(), value)
    }

    pub fn remove(&mut self, key: &str) -> Option<MetaType> {
        self.0.remove(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut meta = Meta::default();

        assert!(meta.is_empty());
        assert!(meta.insert("name", "foo".into()).is_none());
        assert!(meta.insert("name", "foo".into()).is_some());

        // update value
        assert_eq!(meta.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut meta = Meta::default();

        assert!(meta.is_empty());
        meta.insert("name", "foo".into());
        assert_eq!(meta.len(), 1);
    }

    #[test]
    fn test_merge_rhs() {
        let mut meta_a = Meta::default();
        let mut meta_b = Meta::default();
        let mut meta_c = Meta::default();

        meta_a.insert("name", "foo".into());
        meta_a.insert("age", 10.into());
        meta_b.insert("isFun", true.into());

        meta_a.merge_rhs(&meta_b);
        meta_c.merge_rhs(&meta_a);

        assert_eq!(meta_a.len(), 3);
        assert_eq!(meta_c.len(), 3);
    }
}
