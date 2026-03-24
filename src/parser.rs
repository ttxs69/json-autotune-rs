//! JSON parser implementation with multiple optimizations.

use crate::{Error, Value, simd, number};
use std::collections::HashMap;

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
    #[inline]
    fn skip_ws(&mut self) {
        self.pos += simd::skip_whitespace(&self.input[self.pos..]);
    }

    #[inline]
    fn peek(&self) -> Option<u8> { 
        self.input.get(self.pos).copied()
    }

    #[inline]
    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        match self.peek() {
            Some(b) if b == expected => { self.pos += 1; Ok(()) }
            Some(b) => Err(Error::new(format!("Expected '{}', found '{}'", expected as char, b as char), self.pos)),
            None => Err(Error::new("Unexpected end of input", self.pos)),
        }
    }

    #[inline]
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

    #[inline]
    fn parse_null(&mut self) -> Result<Value, Error> {
        if self.input[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(Value::Null)
        } else {
            Err(Error::new("Expected 'null'", self.pos))
        }
    }

    #[inline]
    fn parse_true(&mut self) -> Result<Value, Error> {
        if self.input[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(Value::Bool(true))
        } else {
            Err(Error::new("Expected 'true'", self.pos))
        }
    }

    #[inline]
    fn parse_false(&mut self) -> Result<Value, Error> {
        if self.input[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(Value::Bool(false))
        } else {
            Err(Error::new("Expected 'false'", self.pos))
        }
    }

    #[inline]
    fn parse_string(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip opening quote
        let remaining = &self.input[self.pos..];
        
        let (end, has_escapes) = simd::find_string_end(remaining)
            .ok_or_else(|| Error::new("Unterminated string", self.pos))?;
        
        let raw = &remaining[..end];
        
        // Fast path: no escapes, direct UTF-8 copy
        let s = if !has_escapes {
            // Safety: JSON strings are valid UTF-8
            unsafe { std::str::from_utf8_unchecked(raw) }.to_string()
        } else {
            self.unescape(raw)?
        };
        
        self.pos += end + 1; // +1 for closing quote
        Ok(Value::String(s))
    }

    fn unescape(&self, raw: &[u8]) -> Result<String, Error> {
        let mut result = String::with_capacity(raw.len());
        let mut i = 0;
        
        while i < raw.len() {
            if raw[i] == b'\\' && i + 1 < raw.len() {
                match raw[i + 1] {
                    b'"' => result.push('"'),
                    b'\\' => result.push('\\'),
                    b'/' => result.push('/'),
                    b'b' => result.push('\x08'),
                    b'f' => result.push('\x0c'),
                    b'n' => result.push('\n'),
                    b'r' => result.push('\r'),
                    b't' => result.push('\t'),
                    b'u' => {
                        if i + 5 >= raw.len() {
                            return Err(Error::new("Invalid unicode escape", self.pos + i));
                        }
                        let hex = unsafe { std::str::from_utf8_unchecked(&raw[i + 2..i + 6]) };
                        match u16::from_str_radix(hex, 16) {
                            Ok(code) => {
                                if let Some(c) = char::from_u32(code as u32) {
                                    result.push(c);
                                }
                            }
                            Err(_) => return Err(Error::new("Invalid unicode", self.pos + i)),
                        }
                        i += 4;
                    }
                    b => result.push(b as char),
                }
                i += 2;
            } else {
                result.push(raw[i] as char);
                i += 1;
            }
        }
        Ok(result)
    }

    #[inline]
    fn parse_number(&mut self) -> Result<Value, Error> {
        let remaining = &self.input[self.pos..];
        
        // Try fast integer path first
        if let Some((val, len)) = number::parse_integer(remaining) {
            // Check it's not followed by . or e/E (i.e., not a float)
            if self.pos + len >= self.input.len() || 
               !matches!(self.input[self.pos + len], b'.' | b'e' | b'E') {
                self.pos += len;
                return Ok(Value::Number(val as f64));
            }
        }
        
        // Fallback to full number parsing
        let len = number::skip_number(remaining)
            .ok_or_else(|| Error::new("Invalid number", self.pos))?;
        
        let num_str = unsafe { std::str::from_utf8_unchecked(&self.input[self.pos..self.pos + len]) };
        let num: f64 = num_str.parse().map_err(|_| Error::new("Invalid number", self.pos))?;
        self.pos += len;
        Ok(Value::Number(num))
    }

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

    fn parse_object(&mut self) -> Result<Value, Error> {
        self.pos += 1; // skip {
        self.skip_ws();
        
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(HashMap::new()));
        }

        let mut obj = HashMap::with_capacity(16);

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
}