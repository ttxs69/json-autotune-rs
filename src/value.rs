use hashbrown::HashMap;
use smartstring::SmartString;

// Use SmartString for inline small strings (<= 23 bytes on 64-bit)
// This avoids heap allocation for short strings
pub type JsonString = SmartString<smartstring::LazyCompact>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(JsonString),
    Array(Vec<Value>),
    Object(HashMap<JsonString, Value>),
}

impl Value {
    pub fn is_null(&self) -> bool { matches!(self, Value::Null) }
    pub fn is_bool(&self) -> bool { matches!(self, Value::Bool(_)) }
    pub fn is_number(&self) -> bool { matches!(self, Value::Number(_)) }
    pub fn is_string(&self) -> bool { matches!(self, Value::String(_)) }
    pub fn is_array(&self) -> bool { matches!(self, Value::Array(_)) }
    pub fn is_object(&self) -> bool { matches!(self, Value::Object(_)) }

    pub fn as_bool(&self) -> Option<bool> {
        match self { Value::Bool(b) => Some(*b), _ => None }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self { Value::Number(n) => Some(*n), _ => None }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().filter(|&n| n >= 0.0 && n < u64::MAX as f64).map(|n| n as u64)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self { Value::String(s) => Some(s.as_ref()), _ => None }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self { Value::Array(a) => Some(a), _ => None }
    }

    pub fn as_object(&self) -> Option<&HashMap<JsonString, Value>> {
        match self { Value::Object(o) => Some(o), _ => None }
    }
}

impl std::ops::Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        static NULL: Value = Value::Null;
        match self { Value::Object(o) => o.get(key).unwrap_or(&NULL), _ => &NULL }
    }
}

impl std::ops::Index<usize> for Value {
    type Output = Value;
    fn index(&self, idx: usize) -> &Self::Output {
        static NULL: Value = Value::Null;
        match self { Value::Array(a) => a.get(idx).unwrap_or(&NULL), _ => &NULL }
    }
}

impl Default for Value {
    fn default() -> Self { Value::Null }
}