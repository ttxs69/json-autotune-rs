//! JSON parser - optimized with zero-copy and bulk operations

use crate::{Error, Value, simd, number};
use rustc_hash::FxHashMap;

pub fn parse(input: &str) -> Result<Value, Error> {
    let bytes = input.as_bytes();
    let mut parser = Parser { input: bytes, pos: 0 };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos < parser.input.len() {
        return Err(Error::new("Unexpected characters after JSON value", parser.pos));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    #[inline(always)]
    fn skip_ws(&mut self) {
        self.pos += simd::skip_whitespace(&self.input[self.pos..]);
    }

    #[inline(always)]
    fn peek(&self) -> Option<u8> { 
        self.input.get(self.pos).copied()
    }

    #[inline(always)]
    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        if self.peek() == Some(expected) { 
            self.pos += 1; 
            Ok(()) 
        } else {
            Err(Error::new(format!("Expected '{}'", expected as char), self.pos))
        }
    }

    #[inline(always)]
    fn parse_value(&mut self) -> Result<Value, Error> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => self.parse_null(),
            Some(b't') => self.parse_true(),
            Some(b'f') => self.parse_false(),
            Some(b'"') => self.parse_string(),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(Error::new(format!("Unexpected '{}'", b as char), self.pos)),
            None => Err(Error::new("Unexpected end of input", self.pos)),
        }
    }

    #[inline(always)]
    fn parse_null(&mut self) -> Result<Value, Error> {
        if self.input[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(Value::Null)
        } else {
            Err(Error::new("Expected 'null'", self.pos))
        }
    }

    #[inline(always)]
    fn parse_true(&mut self) -> Result<Value, Error> {
        if self.input[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(Value::Bool(true))
        } else {
            Err(Error::new("Expected 'true'", self.pos))
        }
    }

    #[inline(always)]
    fn parse_false(&mut self) -> Result<Value, Error> {
        if self.input[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(Value::Bool(false))
        } else {
            Err(Error::new("Expected 'false'", self.pos))
        }
    }

    #[inline(always)]
    fn parse_string(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip opening quote
        let remaining = &self.input[self.pos..];
        
        let (end, has_escapes) = simd::find_string_end(remaining)
            .ok_or_else(|| Error::new("Unterminated string", self.pos))?;
        
        let raw = &remaining[..end];
        self.pos += end + 1;
        
        // Fast path: no escapes - direct string creation
        if !has_escapes {
            // Safety: JSON strings are valid UTF-8
            let s = unsafe { 
                std::str::from_utf8_unchecked(raw).to_owned()
            };
            return Ok(Value::String(s));
        }
        
        // Slow path: unescape
        self.unescape_fast(raw)
    }

    /// Fast unescape with minimal allocations
    fn unescape_fast(&self, raw: &[u8]) -> Result<Value, Error> {
        // Pre-scan to estimate output size
        let mut backslash_count = 0;
        for &b in raw.iter().step_by(8) {
            if b == b'\\' { backslash_count += 1; }
        }
        for &b in raw.iter().skip(1).step_by(8) {
            if b == b'\\' { backslash_count += 1; }
        }
        
        let mut result = Vec::with_capacity(raw.len() - backslash_count);
        let mut i = 0;
        
        while i < raw.len() {
            if raw[i] == b'\\' && i + 1 < raw.len() {
                let escaped = match raw[i + 1] {
                    b'"' => b'"',
                    b'\\' => b'\\',
                    b'/' => b'/',
                    b'b' => 0x08,
                    b'f' => 0x0C,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'u' => {
                        if i + 5 >= raw.len() {
                            return Err(Error::new("Invalid unicode escape", self.pos + i));
                        }
                        // Fast hex parse
                        let h1 = (raw[i+2] as char).to_digit(16);
                        let h2 = (raw[i+3] as char).to_digit(16);
                        let h3 = (raw[i+4] as char).to_digit(16);
                        let h4 = (raw[i+5] as char).to_digit(16);
                        
                        match (h1, h2, h3, h4) {
                            (Some(d1), Some(d2), Some(d3), Some(d4)) => {
                                let code = (d1 << 12) | (d2 << 8) | (d3 << 4) | d4;
                                let c = char::from_u32(code as u32).unwrap_or('\u{FFFD}');
                                let mut buf = [0u8; 4];
                                let bytes = c.encode_utf8(&mut buf);
                                result.extend_from_slice(bytes.as_bytes());
                            }
                            _ => return Err(Error::new("Invalid unicode", self.pos + i)),
                        }
                        i += 6;
                        continue;
                    }
                    b => b,
                };
                result.push(escaped);
                i += 2;
            } else {
                result.push(raw[i]);
                i += 1;
            }
        }
        
        // Safety: result is valid UTF-8
        let s = unsafe { String::from_utf8_unchecked(result) };
        Ok(Value::String(s))
    }

    #[inline(always)]
    fn parse_number(&mut self) -> Result<Value, Error> {
        let remaining = &self.input[self.pos..];
        
        // Fast integer path
        if let Some((val, len)) = number::parse_integer(remaining) {
            let next_pos = self.pos + len;
            if next_pos >= self.input.len() || 
               !matches!(self.input[next_pos], b'.' | b'e' | b'E') {
                self.pos += len;
                return Ok(Value::Number(val as f64));
            }
        }
        
        // Float path
        let len = number::skip_number(remaining)
            .ok_or_else(|| Error::new("Invalid number", self.pos))?;
        
        let num_str = unsafe { std::str::from_utf8_unchecked(&self.input[self.pos..self.pos + len]) };
        let num: f64 = num_str.parse().map_err(|_| Error::new("Invalid number", self.pos))?;
        self.pos += len;
        Ok(Value::Number(num))
    }

    #[inline(always)]
    fn parse_array(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip [
        self.skip_ws();
        
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(Vec::new()));
        }

        let mut arr = Vec::with_capacity(16);

        loop {
            arr.push(self.parse_value()?);
            self.skip_ws();
            
            match self.peek() {
                Some(b',') => { self.pos += 1; self.skip_ws(); }
                Some(b']') => { self.pos += 1; break; }
                _ => return Err(Error::new("Expected ',' or ']'", self.pos)),
            }
        }
        
        Ok(Value::Array(arr))
    }

    #[inline(always)]
    fn parse_object(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip {
        self.skip_ws();
        
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(FxHashMap::default()));
        }

        let mut obj = FxHashMap::with_capacity_and_hasher(16, Default::default());

        loop {
            if self.peek() != Some(b'"') {
                return Err(Error::new("Expected string key", self.pos));
            }
            
            let key = match self.parse_string()? {
                Value::String(s) => s,
                _ => unreachable!(),
            };
            
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            
            obj.insert(key, self.parse_value()?);
            self.skip_ws();
            
            match self.peek() {
                Some(b',') => { self.pos += 1; self.skip_ws(); }
                Some(b'}') => { self.pos += 1; break; }
                _ => return Err(Error::new("Expected ',' or '}'", self.pos)),
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
    #[test] fn test_negative() { assert_eq!(parse("-17").unwrap(), Value::Number(-17.0)); }
    #[test] fn test_float() { assert_eq!(parse("3.14").unwrap(), Value::Number(3.14)); }
    #[test] fn test_string() { assert_eq!(parse(r#""hello""#).unwrap(), Value::String("hello".into())); }
    #[test] fn test_escaped() { assert_eq!(parse(r#""hello\nworld""#).unwrap(), Value::String("hello\nworld".into())); }
    #[test] fn test_array() { assert!(parse("[1,2,3]").unwrap().is_array()); }
    #[test] fn test_object() { assert!(parse(r#"{"a":1}"#).unwrap().is_object()); }
    #[test] fn test_large() {
        let json = (0..1000).map(|i| format!(r#"{{"id":{}}}"#, i)).collect::<Vec<_>>().join(",");
        let result = parse(&format!("[{}]", json)).unwrap();
        assert!(result.is_array());
    }
}