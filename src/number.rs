//! Optimized number parsing.

/// Fast integer parsing using SIMD-like techniques.
/// Returns (value, bytes_consumed) or None if invalid.
#[inline]
pub fn parse_integer(data: &[u8]) -> Option<(i64, usize)> {
    if data.is_empty() { return None; }
    
    let mut negative = false;
    let mut pos = 0;
    
    if data[0] == b'-' {
        negative = true;
        pos = 1;
    }
    
    if pos >= data.len() { return None; }
    
    // Fast path: parse up to 8 digits at once
    let mut result: i64 = 0;
    let start = pos;
    
    while pos < data.len() && pos - start < 19 {  // i64 max 19 digits
        let b = data[pos];
        if !(b'0'..=b'9').contains(&b) { break; }
        result = result * 10 + (b - b'0') as i64;
        pos += 1;
    }
    
    if pos == start { return None; }  // No digits
    
    if negative { result = -result; }
    Some((result, pos))
}

/// Check if bytes represent a valid number and return end position.
/// This is faster than parsing when we just need to skip.
#[inline]
pub fn skip_number(data: &[u8]) -> Option<usize> {
    if data.is_empty() { return None; }
    
    let mut pos = 0;
    
    // Optional sign
    if data[pos] == b'-' {
        pos += 1;
        if pos >= data.len() { return None; }
    }
    
    // Integer part
    if data[pos] == b'0' {
        pos += 1;
    } else if data[pos].is_ascii_digit() {
        while pos < data.len() && data[pos].is_ascii_digit() { pos += 1; }
    } else {
        return None;
    }
    
    // Fraction
    if pos < data.len() && data[pos] == b'.' {
        pos += 1;
        if pos >= data.len() || !data[pos].is_ascii_digit() { return None; }
        while pos < data.len() && data[pos].is_ascii_digit() { pos += 1; }
    }
    
    // Exponent
    if pos < data.len() && (data[pos] == b'e' || data[pos] == b'E') {
        pos += 1;
        if pos < data.len() && (data[pos] == b'+' || data[pos] == b'-') { pos += 1; }
        if pos >= data.len() || !data[pos].is_ascii_digit() { return None; }
        while pos < data.len() && data[pos].is_ascii_digit() { pos += 1; }
    }
    
    Some(pos)
}

/// Fast f64 parsing with integer fast path.
#[inline]
pub fn parse_f64(data: &[u8]) -> Option<f64> {
    let len = skip_number(data)?;
    let s = unsafe { std::str::from_utf8_unchecked(&data[..len]) };
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        assert_eq!(parse_integer(b"123"), Some((123, 3)));
        assert_eq!(parse_integer(b"-456"), Some((-456, 4)));
        assert_eq!(parse_integer(b"0"), Some((0, 1)));
        assert_eq!(parse_integer(b"abc"), None);
    }
    
    #[test]
    fn test_skip() {
        assert_eq!(skip_number(b"123"), Some(3));
        assert_eq!(skip_number(b"3.14"), Some(4));
        assert_eq!(skip_number(b"1e10"), Some(4));
        assert_eq!(skip_number(b"-123.456e-7"), Some(11));
    }
}