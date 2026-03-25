//! Optimized number parsing.

// Lookup table for digit values (0-9) and invalid marker (255)
const DIGIT: [u8; 256] = {
    let mut table = [255u8; 256];
    let mut i = 0;
    while i < 10 {
        table[b'0' as usize + i] = i as u8;
        i += 1;
    }
    table
};

/// Fast integer parsing - optimized for common cases.
/// Returns (value, bytes_consumed) or None if invalid.
#[inline(always)]
pub fn parse_integer(data: &[u8]) -> Option<(i64, usize)> {
    if data.is_empty() { return None; }
    
    let mut negative = false;
    let mut pos = 0;
    
    if data[0] == b'-' {
        negative = true;
        pos = 1;
    }
    
    if pos >= data.len() { return None; }
    
    // Use lookup table for digit parsing
    let first = DIGIT[data[pos] as usize];
    if first == 255 { return None; }
    
    let mut result: i64 = first as i64;
    pos += 1;
    
    // Unrolled loop for common case (1-4 digits)
    if pos < data.len() {
        let d = DIGIT[data[pos] as usize];
        if d != 255 {
            result = result * 10 + d as i64;
            pos += 1;
            if pos < data.len() {
                let d = DIGIT[data[pos] as usize];
                if d != 255 {
                    result = result * 10 + d as i64;
                    pos += 1;
                    if pos < data.len() {
                        let d = DIGIT[data[pos] as usize];
                        if d != 255 {
                            result = result * 10 + d as i64;
                            pos += 1;
                        }
                    }
                }
            }
        }
    }
    
    // Continue for longer numbers
    while pos < data.len() && pos < 19 {
        let d = DIGIT[data[pos] as usize];
        if d == 255 { break; }
        result = result * 10 + d as i64;
        pos += 1;
    }
    
    if negative { result = -result; }
    Some((result, pos))
}

/// Check if bytes represent a valid number and return end position.
/// This is faster than parsing when we just need to skip.
#[inline(always)]
pub fn skip_number(data: &[u8]) -> Option<usize> {
    if data.is_empty() { return None; }
    
    let mut pos = 0;
    
    // Optional sign
    if data[pos] == b'-' {
        pos += 1;
        if pos >= data.len() { return None; }
    }
    
    // Integer part - use direct byte comparison for speed
    let first = data[pos];
    if first == b'0' {
        pos += 1;
    } else if first >= b'0' && first <= b'9' {
        pos += 1;
        while pos < data.len() {
            let b = data[pos];
            if b < b'0' || b > b'9' { break; }
            pos += 1;
        }
    } else {
        return None;
    }
    
    // Fraction
    if pos < data.len() && data[pos] == b'.' {
        pos += 1;
        if pos >= data.len() { return None; }
        let b = data[pos];
        if b < b'0' || b > b'9' { return None; }
        pos += 1;
        while pos < data.len() {
            let b = data[pos];
            if b < b'0' || b > b'9' { break; }
            pos += 1;
        }
    }
    
    // Exponent
    if pos < data.len() && (data[pos] == b'e' || data[pos] == b'E') {
        pos += 1;
        if pos < data.len() && (data[pos] == b'+' || data[pos] == b'-') { pos += 1; }
        if pos >= data.len() { return None; }
        let b = data[pos];
        if b < b'0' || b > b'9' { return None; }
        pos += 1;
        while pos < data.len() {
            let b = data[pos];
            if b < b'0' || b > b'9' { break; }
            pos += 1;
        }
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