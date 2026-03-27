//! JSON parser - highly optimized

use crate::{Error, Value, simd, number};
use rustc_hash::FxHashMap;

// Lookup table for keyword matching (faster than memcmp for short words)
const KEYWORD_NULL: u32 = 0x6c6c756e; // "null" as u32 (little-endian)
const KEYWORD_TRUE: u32 = 0x65757274; // "true" as u32 (little-endian)

pub fn parse(input: &str) -> Result<Value, Error> {
    let bytes = input.as_bytes();
    
    // Minimal Parser struct for fast parsing
    let mut p = Parser { input: bytes, pos: 0 };
    let v = p.parse_value()?;
    // Inline skip_ws check for trailing data
    p.pos += simd::skip_whitespace(unsafe { p.input.get_unchecked(p.pos..) });
    if p.pos < p.input.len() {
        return Err(Error::new("Trailing data", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    #[inline(always)]
    fn parse_value(&mut self) -> Result<Value, Error> {
        // Inline skip_ws with get_unchecked
        self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
        self.parse_value_inner()
    }

    /// Parse value without skipping whitespace first (caller already did)
    #[inline(always)]
    fn parse_value_inner(&mut self) -> Result<Value, Error> {
        // Use get_unchecked for faster byte access
        let b = unsafe { *self.input.get_unchecked(self.pos) };
        
        // Fast dispatch - most common first
        match b {
            b'"' => self.parse_string(),
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'0'..=b'9' | b'-' => self.parse_number(),
            b't' => self.parse_true(),
            b'f' => self.parse_false(),
            b'n' => self.parse_null(),
            _ => Err(Error::new("Invalid char", self.pos)),
        }
    }

    #[inline(always)]
    fn parse_null(&mut self) -> Result<Value, Error> {
        // Fast path: read 4 bytes as u32 and compare
        if self.pos + 4 <= self.input.len() {
            let word = unsafe {
                u32::from_le_bytes(*(self.input.as_ptr().add(self.pos) as *const [u8; 4]))
            };
            if word == KEYWORD_NULL {
                self.pos += 4;
                return Ok(Value::Null);
            }
        }
        Err(Error::new("Expected null", self.pos))
    }

    #[inline(always)]
    fn parse_true(&mut self) -> Result<Value, Error> {
        if self.pos + 4 <= self.input.len() {
            let word = unsafe {
                u32::from_le_bytes(*(self.input.as_ptr().add(self.pos) as *const [u8; 4]))
            };
            if word == KEYWORD_TRUE {
                self.pos += 4;
                return Ok(Value::Bool(true));
            }
        }
        Err(Error::new("Expected true", self.pos))
    }

    #[inline(always)]
    fn parse_false(&mut self) -> Result<Value, Error> {
        if self.pos + 5 <= self.input.len() {
            let first = unsafe { *self.input.get_unchecked(self.pos) };
            if first == b'f' {
                let suffix = unsafe {
                    u32::from_le_bytes(*(self.input.as_ptr().add(self.pos + 1) as *const [u8; 4]))
                };
                if suffix == 0x65736c61 { // "alse" in little-endian
                    self.pos += 5;
                    return Ok(Value::Bool(false));
                }
            }
        }
        Err(Error::new("Expected false", self.pos))
    }

    #[inline(always)]
    fn parse_string(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip quote
        
        // Use get_unchecked for faster slice access
        let remaining = unsafe { self.input.get_unchecked(self.pos..) };
        
        let (end, has_escapes) = simd::find_string_end(remaining)
            .ok_or_else(|| Error::new("Unterminated string", self.pos))?;
        
        let raw = unsafe { remaining.get_unchecked(..end) };
        self.pos += end + 1;
        
        if !has_escapes {
            // Fast path: directly create String from bytes
            let mut s = String::with_capacity(end);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    raw.as_ptr(),
                    s.as_mut_ptr() as *mut u8,
                    end
                );
                s.as_mut_vec().set_len(end);
            }
            return Ok(Value::String(s));
        }
        
        self.unescape(raw)
    }

    #[cold]
    fn unescape(&self, raw: &[u8]) -> Result<Value, Error> {
        let mut result = Vec::with_capacity(raw.len());
        let mut i = 0;
        
        while i < raw.len() {
            if raw[i] == b'\\' && i + 1 < raw.len() {
                match raw[i + 1] {
                    b'"' => result.push(b'"'),
                    b'\\' => result.push(b'\\'),
                    b'/' => result.push(b'/'),
                    b'b' => result.push(0x08),
                    b'f' => result.push(0x0C),
                    b'n' => result.push(b'\n'),
                    b'r' => result.push(b'\r'),
                    b't' => result.push(b'\t'),
                    b'u' => {
                        if i + 5 >= raw.len() {
                            return Err(Error::new("Invalid unicode", self.pos + i));
                        }
                        let h = |b: u8| (b as char).to_digit(16);
                        match (h(raw[i+2]), h(raw[i+3]), h(raw[i+4]), h(raw[i+5])) {
                            (Some(d1), Some(d2), Some(d3), Some(d4)) => {
                                let code = (d1 << 12) | (d2 << 8) | (d3 << 4) | d4;
                                let c = char::from_u32(code as u32).unwrap_or('\u{FFFD}');
                                let mut buf = [0u8; 4];
                                result.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                            }
                            _ => return Err(Error::new("Invalid unicode", self.pos + i)),
                        }
                        i += 6;
                        continue;
                    }
                    b => result.push(b),
                }
                i += 2;
            } else {
                result.push(raw[i]);
                i += 1;
            }
        }
        
        Ok(Value::String(unsafe { String::from_utf8_unchecked(result) }))
    }

    #[inline(always)]
    fn parse_number(&mut self) -> Result<Value, Error> {
        let remaining = unsafe { self.input.get_unchecked(self.pos..) };
        
        // Fast integer path
        if let Some((val, len)) = number::parse_integer(remaining) {
            let next_pos = self.pos + len;
            if next_pos >= self.input.len() {
                self.pos = next_pos;
                return Ok(Value::Number(val as f64));
            }
            let next_byte = unsafe { *self.input.get_unchecked(next_pos) };
            if next_byte != b'.' && next_byte != b'e' && next_byte != b'E' {
                self.pos = next_pos;
                return Ok(Value::Number(val as f64));
            }
        }
        
        // Use lexical-core for fast float parsing
        let len = number::skip_number(remaining)
            .ok_or_else(|| Error::new("Invalid number", self.pos))?;
        
        let s = unsafe { std::str::from_utf8_unchecked(self.input.get_unchecked(self.pos..self.pos + len)) };
        let n: f64 = lexical_core::parse(s.as_bytes())
            .map_err(|_| Error::new("Invalid number", self.pos))?;
        self.pos += len;
        Ok(Value::Number(n))
    }

    #[inline(always)]
    fn parse_array(&mut self) -> Result<Value, Error> {
        self.pos += 1;
        
        // Inline skip_ws
        self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
        
        if self.pos < self.input.len() && unsafe { *self.input.get_unchecked(self.pos) } == b']' {
            self.pos += 1;
            return Ok(Value::Array(Vec::new()));
        }

        // Start with capacity 8 - balance between small and large arrays
        let mut arr = Vec::with_capacity(8);

        loop {
            arr.push(self.parse_value_inner()?);
            
            // Inline skip_ws after value
            self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
            
            let b = unsafe { *self.input.get_unchecked(self.pos) };
            if b == b',' { 
                self.pos += 1;
                // Skip whitespace after comma (rare in compact JSON)
                let next = unsafe { *self.input.get_unchecked(self.pos) };
                if next == b' ' || next == b'\t' || next == b'\n' || next == b'\r' {
                    self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
                }
            } else if b == b']' { 
                self.pos += 1; 
                break;
            } else {
                return Err(Error::new("Expected ',' or ']'", self.pos));
            }
        }
        
        Ok(Value::Array(arr))
    }

    #[inline(always)]
    fn parse_object(&mut self) -> Result<Value, Error> {
        self.pos += 1;
        self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
        
        if self.pos < self.input.len() && unsafe { *self.input.get_unchecked(self.pos) } == b'}' {
            self.pos += 1;
            return Ok(Value::Object(FxHashMap::default()));
        }

        // Pre-allocate with capacity 3 - most JSON objects are small (avg ~3 fields)
        let mut obj = FxHashMap::with_capacity_and_hasher(3, Default::default());

        loop {
            if unsafe { *self.input.get_unchecked(self.pos) } != b'"' {
                return Err(Error::new("Expected string key", self.pos));
            }
            
            let key = match self.parse_string()? {
                Value::String(s) => s,
                _ => unreachable!(),
            };
            
            self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
            
            if unsafe { *self.input.get_unchecked(self.pos) } != b':' {
                return Err(Error::new("Expected ':'", self.pos));
            }
            self.pos += 1;
            self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
            
            obj.insert(key, self.parse_value_inner()?);
            
            self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
            
            let b = unsafe { *self.input.get_unchecked(self.pos) };
            if b == b',' { 
                self.pos += 1;
                self.pos += simd::skip_whitespace(unsafe { self.input.get_unchecked(self.pos..) });
            } else if b == b'}' { 
                self.pos += 1; 
                break;
            } else {
                return Err(Error::new("Expected ',' or '}'", self.pos));
            }
        }
        
        Ok(Value::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_null() { assert_eq!(parse("null").unwrap(), Value::Null); }
    #[test] fn test_bool() { assert_eq!(parse("true").unwrap(), Value::Bool(true)); }
    #[test] fn test_number() { assert_eq!(parse("42").unwrap(), Value::Number(42.0)); }
    #[test] fn test_string() { assert_eq!(parse(r#""hello""#).unwrap(), Value::String("hello".into())); }
    #[test] fn test_array() { assert!(parse("[1,2,3]").unwrap().is_array()); }
    #[test] fn test_object() { assert!(parse(r#"{"a":1}"#).unwrap().is_object()); }
}