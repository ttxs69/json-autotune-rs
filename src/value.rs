use hashbrown::HashMap;
use smartstring::SmartString;
use foldhash::fast::FixedState;

// Use SmartString for inline small strings (<= 23 bytes on 64-bit)
pub type JsonString = SmartString<smartstring::LazyCompact>;

// Small object optimization: use inline array for objects with <= 3 fields
// This avoids heap allocation entirely for very small objects
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Tiny(Box<[(JsonString, Value); 3]>),  // <= 3 fields, inline storage
    Small(Vec<(JsonString, Value)>),  // 4-8 fields
    Large(HashMap<JsonString, Value, FixedState>),  // > 8 fields
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(JsonString),
    Array(Vec<Value>),
    Object(Object),
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

    pub fn as_object_small(&self) -> Option<&Vec<(JsonString, Value)>> {
        match self { 
            Value::Object(Object::Small(v)) => Some(v), 
            _ => None 
        }
    }
    
    pub fn as_object_large(&self) -> Option<&HashMap<JsonString, Value, FixedState>> {
        match self { 
            Value::Object(Object::Large(m)) => Some(m), 
            _ => None 
        }
    }
}

impl std::ops::Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        static NULL: Value = Value::Null;
        match self { 
            Value::Object(Object::Tiny(arr)) => {
                arr.iter().find(|(k, _)| k.as_str() == key).map(|(_, v)| v).unwrap_or(&NULL)
            }
            Value::Object(Object::Small(v)) => {
                v.iter().find(|(k, _)| k.as_str() == key).map(|(_, v)| v).unwrap_or(&NULL)
            }
            Value::Object(Object::Large(m)) => m.get(key).unwrap_or(&NULL),
            _ => &NULL,
        }
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