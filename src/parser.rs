//! JSON parser - highly optimized

use crate::{Error, Value, simd, number};
use rustc_hash::FxHashMap;

// Lookup table for keyword matching (faster than memcmp for short words)
const KEYWORD_NULL: u32 = 0x6c6c756e; // "null" as u32 (little-endian)
const KEYWORD_TRUE: u32 = 0x65757274; // "true" as u32 (little-endian)

pub fn parse(input: &str) -> Result<Value, Error> {
    let bytes = input.as_bytes();
    
    let (arr_cap, obj_cap) = if bytes.len() > 4096 {
        estimate_sizes(bytes)
    } else {
        (16, 16)
    };
    
    let mut p = Parser { input: bytes, pos: 0, arr_cap, obj_cap };
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos < p.input.len() {
        return Err(Error::new("Trailing data", p.pos));
    }
    Ok(v)
}

#[inline]
fn estimate_sizes(data: &[u8]) -> (usize, usize) {
    let mut commas = 0usize;
    let mut containers = 0usize;
    
    for &b in data.iter().step_by(8) {
        match b {
            b'[' | b'{' => containers += 1,
            b',' => commas += 1,
            _ => {}
        }
    }
    
    let avg = if containers > 0 { (commas / containers + 1).min(64) } else { 16 };
    (avg, avg)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    arr_cap: usize,
    obj_cap: usize,
}

impl<'a> Parser<'a> {
    #[inline(always)]
    fn skip_ws(&mut self) {
        let data = &self.input[self.pos..];
        let skip = simd::skip_whitespace(data);
        self.pos += skip;
    }

    #[inline(always)]
    fn peek(&self) -> u8 {
        // Safety: we always check bounds before calling
        unsafe { *self.input.get_unchecked(self.pos) }
    }

    #[inline(always)]
    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    #[inline(always)]
    fn parse_value(&mut self) -> Result<Value, Error> {
        self.skip_ws();
        
        if self.at_end() {
            return Err(Error::new("Unexpected end", self.pos));
        }
        
        let b = self.peek();
        
        // Fast path for common cases with direct dispatch
        match b {
            b'"' => self.parse_string(),
            b'0'..=b'9' => self.parse_number(),
            b'-' => self.parse_number(),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b't' => self.parse_true(),
            b'f' => self.parse_false(),
            b'n' => self.parse_null(),
            _ => Err(Error::new("Invalid char", self.pos)),
        }
    }

    #[inline(always)]
    fn parse_null(&mut self) -> Result<Value, Error> {
        // Fast path: read 4 bytes as u32 and compare
        let remaining = &self.input[self.pos..];
        if remaining.len() >= 4 {
            let word = unsafe {
                u32::from_le_bytes(*(remaining.as_ptr().add(0) as *const [u8; 4]))
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
        let remaining = &self.input[self.pos..];
        if remaining.len() >= 4 {
            let word = unsafe {
                u32::from_le_bytes(*(remaining.as_ptr().add(0) as *const [u8; 4]))
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
        let remaining = &self.input[self.pos..];
        if remaining.len() >= 5 && remaining[0] == b'f' {
            // Check "alse" part using u16 compare
            let suffix = unsafe {
                u32::from_le_bytes(*(remaining.as_ptr().add(1) as *const [u8; 4]))
            };
            if suffix == 0x65736c61 { // "alse" in little-endian
                self.pos += 5;
                return Ok(Value::Bool(false));
            }
        }
        Err(Error::new("Expected false", self.pos))
    }

    #[inline(always)]
    fn parse_string(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip quote
        let remaining = &self.input[self.pos..];
        
        let (end, has_escapes) = simd::find_string_end(remaining)
            .ok_or_else(|| Error::new("Unterminated string", self.pos))?;
        
        let raw = &remaining[..end];
        self.pos += end + 1;
        
        if !has_escapes {
            let s = unsafe { std::str::from_utf8_unchecked(raw).to_owned() };
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
        let remaining = &self.input[self.pos..];
        
        if let Some((val, len)) = number::parse_integer(remaining) {
            if self.pos + len >= self.input.len() || 
               !matches!(self.input[self.pos + len], b'.' | b'e' | b'E') {
                self.pos += len;
                return Ok(Value::Number(val as f64));
            }
        }
        
        let len = number::skip_number(remaining)
            .ok_or_else(|| Error::new("Invalid number", self.pos))?;
        
        let s = unsafe { std::str::from_utf8_unchecked(&self.input[self.pos..self.pos + len]) };
        let n: f64 = s.parse().map_err(|_| Error::new("Invalid number", self.pos))?;
        self.pos += len;
        Ok(Value::Number(n))
    }

    #[inline(always)]
    fn parse_array(&mut self) -> Result<Value, Error> {
        self.pos += 1;
        
        // Inline skip_ws for empty array check
        let skip = simd::skip_whitespace(&self.input[self.pos..]);
        self.pos += skip;
        
        if self.pos < self.input.len() && self.input[self.pos] == b']' {
            self.pos += 1;
            return Ok(Value::Array(Vec::new()));
        }

        let mut arr = Vec::with_capacity(self.arr_cap);

        loop {
            arr.push(self.parse_value()?);
            
            // Inline skip_ws after value
            let skip = simd::skip_whitespace(&self.input[self.pos..]);
            self.pos += skip;
            
            if self.pos >= self.input.len() {
                return Err(Error::new("Unclosed array", self.pos));
            }
            
            let b = self.input[self.pos];
            if b == b',' { 
                self.pos += 1; 
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
        
        // Inline skip_ws for empty object check
        let skip = simd::skip_whitespace(&self.input[self.pos..]);
        self.pos += skip;
        
        if self.pos < self.input.len() && self.input[self.pos] == b'}' {
            self.pos += 1;
            return Ok(Value::Object(FxHashMap::default()));
        }

        let mut obj = FxHashMap::with_capacity_and_hasher(self.obj_cap, Default::default());

        loop {
            if self.pos >= self.input.len() || self.input[self.pos] != b'"' {
                return Err(Error::new("Expected string key", self.pos));
            }
            
            let key = match self.parse_string()? {
                Value::String(s) => s,
                _ => unreachable!(),
            };
            
            // Inline skip_ws after key
            let skip = simd::skip_whitespace(&self.input[self.pos..]);
            self.pos += skip;
            
            if self.pos >= self.input.len() || self.input[self.pos] != b':' {
                return Err(Error::new("Expected ':'", self.pos));
            }
            self.pos += 1;
            
            let value = self.parse_value()?;
            obj.insert(key, value);
            
            // Inline skip_ws after value
            let skip = simd::skip_whitespace(&self.input[self.pos..]);
            self.pos += skip;
            
            if self.pos >= self.input.len() {
                return Err(Error::new("Unclosed object", self.pos));
            }
            
            let b = self.input[self.pos];
            if b == b',' { 
                self.pos += 1; 
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