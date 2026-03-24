//! JSON parser implementation.

use crate::{Error, Value, simd};
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
    fn skip_ws(&mut self) {
        self.pos += simd::skip_whitespace(&self.input[self.pos..]);
    }

    fn peek(&self) -> Option<u8> { self.input.get(self.pos).copied() }

    fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        match self.advance() {
            Some(b) if b == expected => Ok(()),
            Some(b) => Err(Error::new(format!("Expected '{}', found '{}'", expected as char, b as char), self.pos - 1)),
            None => Err(Error::new("Unexpected end of input", self.pos)),
        }
    }

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

    fn parse_null(&mut self) -> Result<Value, Error> {
        if self.consume(b"null") { Ok(Value::Null) } else { Err(Error::new("Expected 'null'", self.pos)) }
    }

    fn parse_true(&mut self) -> Result<Value, Error> {
        if self.consume(b"true") { Ok(Value::Bool(true)) } else { Err(Error::new("Expected 'true'", self.pos)) }
    }

    fn parse_false(&mut self) -> Result<Value, Error> {
        if self.consume(b"false") { Ok(Value::Bool(false)) } else { Err(Error::new("Expected 'false'", self.pos)) }
    }

    fn consume(&mut self, literal: &[u8]) -> bool {
        if self.input[self.pos..].starts_with(literal) {
            self.pos += literal.len();
            true
        } else { false }
    }

    fn parse_string(&mut self) -> Result<Value, Error> {
        self.expect(b'"')?;
        let remaining = &self.input[self.pos..];
        let end = simd::find_string_end(remaining).ok_or_else(|| Error::new("Unterminated string", self.pos))?;
        let raw = &remaining[..end];
        let s = self.unescape(raw)?;
        self.pos += end + 1;
        Ok(Value::String(s))
    }

    fn unescape(&self, raw: &[u8]) -> Result<String, Error> {
        let mut result = String::with_capacity(raw.len());
        let mut i = 0;
        while i < raw.len() {
            if raw[i] == b'\\' && i + 1 < raw.len() {
                match raw[i + 1] {
                    b'"' => result.push('"'), b'\\' => result.push('\\'), b'/' => result.push('/'),
                    b'b' => result.push('\x08'), b'f' => result.push('\x0c'),
                    b'n' => result.push('\n'), b'r' => result.push('\r'), b't' => result.push('\t'),
                    b'u' => {
                        if i + 5 >= raw.len() { return Err(Error::new("Invalid unicode escape", self.pos + i)); }
                        let hex = &raw[i + 2..i + 6];
                        let code = u16::from_str_radix(std::str::from_utf8(hex).map_err(|_| Error::new("Invalid unicode", self.pos + i))?, 16).map_err(|_| Error::new("Invalid unicode", self.pos + i))?;
                        if let Some(c) = char::from_u32(code as u32) { result.push(c); }
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

    fn parse_number(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        if self.peek() == Some(b'-') { self.advance(); }
        match self.peek() {
            Some(b'0') => { self.advance(); }
            Some(b'1'..=b'9') => { while let Some(b'0'..=b'9') = self.peek() { self.advance(); } }
            _ => return Err(Error::new("Invalid number", start)),
        }
        if self.peek() == Some(b'.') {
            self.advance();
            if !matches!(self.peek(), Some(b'0'..=b'9')) { return Err(Error::new("Invalid fraction", self.pos)); }
            while let Some(b'0'..=b'9') = self.peek() { self.advance(); }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) { self.advance(); }
            if !matches!(self.peek(), Some(b'0'..=b'9')) { return Err(Error::new("Invalid exponent", self.pos)); }
            while let Some(b'0'..=b'9') = self.peek() { self.advance(); }
        }
        let num_str = std::str::from_utf8(&self.input[start..self.pos]).map_err(|_| Error::new("Invalid UTF-8", start))?;
        let num: f64 = num_str.parse().map_err(|_| Error::new("Invalid number", start))?;
        Ok(Value::Number(num))
    }

    fn parse_array(&mut self) -> Result<Value, Error> {
        self.expect(b'[')?;
        self.skip_ws();
        if self.peek() == Some(b']') { self.advance(); return Ok(Value::Array(Vec::new())); }

        let mut arr = Vec::new();
        loop {
            arr.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => { self.advance(); self.skip_ws(); }
                Some(b']') => { self.advance(); break; }
                _ => return Err(Error::new("Expected ',' or ']'", self.pos)),
            }
        }
        Ok(Value::Array(arr))
    }

    fn parse_object(&mut self) -> Result<Value, Error> {
        self.expect(b'{')?;
        self.skip_ws();
        if self.peek() == Some(b'}') { self.advance(); return Ok(Value::Object(HashMap::new())); }

        let mut obj = HashMap::new();
        loop {
            if self.peek() != Some(b'"') { return Err(Error::new("Expected string key", self.pos)); }
            let key = match self.parse_string()? { Value::String(s) => s, _ => unreachable!() };
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            obj.insert(key, self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => { self.advance(); self.skip_ws(); }
                Some(b'}') => { self.advance(); break; }
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
    #[test] fn test_string() { assert_eq!(parse(r#""hello""#).unwrap(), Value::String("hello".into())); }
    #[test] fn test_array() { assert!(parse("[1,2,3]").unwrap().is_array()); }
    #[test] fn test_object() { assert!(parse(r#"{"a":1}"#).unwrap().is_object()); }
}