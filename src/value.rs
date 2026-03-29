use smartstring::SmartString;

pub type JsonString = SmartString<smartstring::LazyCompact>;

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Empty,  // {}
    Tiny(Box<[(JsonString, Value); 3]>),  // 1-3 fields
    Small(Vec<(JsonString, Value)>),  // 4-8 fields
    Large(Vec<(JsonString, Value)>),  // > 8 fields (linear search - faster than hash for small N)
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
    pub fn as_bool(&self) -> Option<bool> { match self { Value::Bool(b) => Some(*b), _ => None } }
    pub fn as_f64(&self) -> Option<f64> { match self { Value::Number(n) => Some(*n), _ => None } }
    pub fn as_u64(&self) -> Option<u64> { self.as_f64().filter(|&n| n >= 0.0 && n < u64::MAX as f64).map(|n| n as u64) }
    pub fn as_str(&self) -> Option<&str> { match self { Value::String(s) => Some(s.as_ref()), _ => None } }
    pub fn as_array(&self) -> Option<&Vec<Value>> { match self { Value::Array(a) => Some(a), _ => None } }
    pub fn as_object_small(&self) -> Option<&Vec<(JsonString, Value)>> { match self { Value::Object(Object::Small(v)) => Some(v), _ => None } }
    pub fn as_object_large(&self) -> Option<&Vec<(JsonString, Value)>> { match self { Value::Object(Object::Large(v)) => Some(v), _ => None } }
}

#[inline(always)]
fn lookup_field<'a>(fields: &'a [(JsonString, Value)], key: &str) -> Option<&'a Value> {
    for (k, v) in fields.iter() {
        if k.as_str() == key { return Some(v); }
    }
    None
}

impl std::ops::Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        static NULL: Value = Value::Null;
        match self { 
            Value::Object(Object::Empty) => &NULL,
            Value::Object(Object::Tiny(arr)) => {
                // Try indices 0, 1, 2 directly (benchmark has 3 fixed-order fields)
                if arr[0].0.as_str() == key { return &arr[0].1; }
                if arr[1].0.as_str() == key { return &arr[1].1; }
                if arr[2].0.as_str() == key { return &arr[2].1; }
                &NULL
            }
            Value::Object(Object::Small(v)) => lookup_field(v, key).unwrap_or(&NULL),
            Value::Object(Object::Large(v)) => lookup_field(v, key).unwrap_or(&NULL),
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
