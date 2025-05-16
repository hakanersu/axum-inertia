// shared_props.rs
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct SharedProps(pub HashMap<String, Value>);

impl SharedProps {
    pub fn insert<K: Into<String>, V: Into<Value>>(&mut self, key: K, value: V) {
        self.0.insert(key.into(), value.into());
    }
}
