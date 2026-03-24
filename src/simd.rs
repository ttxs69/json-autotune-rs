//! SIMD-accelerated JSON primitives.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Skip whitespace using SIMD (16 bytes at a time).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub unsafe fn skip_whitespace_simd(data: &[u8]) -> usize {
    if data.len() < 16 { return skip_whitespace_scalar(data); }

    let spaces = _mm_set1_epi8(0x20);
    let tabs = _mm_set1_epi8(0x09);
    let newlines = _mm_set1_epi8(0x0a);
    let crs = _mm_set1_epi8(0x0d);

    let mut offset = 0;
    let chunks = data.len() / 16;

    for _ in 0..chunks {
        let chunk = _mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i);
        let eq_space = _mm_cmpeq_epi8(chunk, spaces);
        let eq_tab = _mm_cmpeq_epi8(chunk, tabs);
        let eq_newline = _mm_cmpeq_epi8(chunk, newlines);
        let eq_cr = _mm_cmpeq_epi8(chunk, crs);
        let ws = _mm_or_si128(_mm_or_si128(eq_space, eq_tab), _mm_or_si128(eq_newline, eq_cr));
        let mask = _mm_movemask_epi8(ws) as u16;

        let inverted = !mask;
        if inverted == 0 {
            offset += 16;
        } else {
            return offset + inverted.trailing_zeros() as usize;
        }
    }
    offset + skip_whitespace_scalar(&data[offset..])
}

#[cfg(not(target_arch = "x86_64"))]
pub fn skip_whitespace_simd(data: &[u8]) -> usize {
    skip_whitespace_scalar(data)
}

pub fn skip_whitespace_scalar(data: &[u8]) -> usize {
    data.iter().position(|&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r')).unwrap_or(data.len())
}

pub fn skip_whitespace(data: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    { if is_x86_feature_detected!("sse2") { unsafe { skip_whitespace_simd(data) } } else { skip_whitespace_scalar(data) } }
    #[cfg(not(target_arch = "x86_64"))]
    { skip_whitespace_scalar(data) }
}

/// Find string end (closing quote) using SIMD.
/// Returns (position, has_escapes) or None if not found.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub unsafe fn find_string_end_simd(data: &[u8]) -> Option<(usize, bool)> {
    if data.is_empty() { return None; }

    let quotes = _mm_set1_epi8(b'"' as i8);
    let backslashes = _mm_set1_epi8(b'\\' as i8);

    let mut offset = 0;
    let mut escaped = false;
    let mut has_escapes = false;

    while offset + 16 <= data.len() {
        let chunk = _mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i);
        let quote_mask = _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, quotes)) as u16;
        let backslash_mask = _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, backslashes)) as u16;
        
        if backslash_mask != 0 { has_escapes = true; }

        for i in 0..16 {
            if offset + i >= data.len() { return None; }
            if (backslash_mask >> i) & 1 != 0 { escaped = !escaped; }
            else if (quote_mask >> i) & 1 != 0 {
                if !escaped { return Some((offset + i, has_escapes)); }
                escaped = false;
            } else { escaped = false; }
        }
        offset += 16;
    }

    for i in offset..data.len() {
        if data[i] == b'\\' { escaped = !escaped; has_escapes = true; }
        else if data[i] == b'"' && !escaped { return Some((i, has_escapes)); }
        else { escaped = false; }
    }
    None
}

#[cfg(not(target_arch = "x86_64"))]
pub fn find_string_end_simd(data: &[u8]) -> Option<(usize, bool)> {
    find_string_end_scalar(data)
}

pub fn find_string_end_scalar(data: &[u8]) -> Option<(usize, bool)> {
    let mut escaped = false;
    let mut has_escapes = false;
    for (i, &b) in data.iter().enumerate() {
        if b == b'\\' { escaped = !escaped; has_escapes = true; }
        else if b == b'"' && !escaped { return Some((i, has_escapes)); }
        else { escaped = false; }
    }
    None
}

#[inline]
pub fn find_string_end(data: &[u8]) -> Option<(usize, bool)> {
    #[cfg(target_arch = "x86_64")]
    { if is_x86_feature_detected!("sse2") { unsafe { find_string_end_simd(data) } } else { find_string_end_scalar(data) } }
    #[cfg(not(target_arch = "x86_64"))]
    { find_string_end_scalar(data) }
}

/// Check if a byte is a structural character.
#[inline]
pub fn is_structural(b: u8) -> bool {
    matches!(b, b'{' | b'}' | b'[' | b']' | b',' | b':')
}

/// Find next structural character using SIMD.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub unsafe fn find_structural_simd(data: &[u8]) -> Option<usize> {
    if data.len() < 16 {
        return data.iter().position(|&b| is_structural(b));
    }

    let openc = _mm_set1_epi8(b'{' as i8);
    let closec = _mm_set1_epi8(b'}' as i8);
    let openb = _mm_set1_epi8(b'[' as i8);
    let closeb = _mm_set1_epi8(b']' as i8);
    let comma = _mm_set1_epi8(b',' as i8);
    let colon = _mm_set1_epi8(b':' as i8);

    let mut offset = 0;
    let chunks = data.len() / 16;

    for _ in 0..chunks {
        let chunk = _mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i);
        
        let m0 = _mm_or_si128(_mm_cmpeq_epi8(chunk, openc), _mm_cmpeq_epi8(chunk, closec));
        let m1 = _mm_or_si128(_mm_cmpeq_epi8(chunk, openb), _mm_cmpeq_epi8(chunk, closeb));
        let m2 = _mm_or_si128(_mm_cmpeq_epi8(chunk, comma), _mm_cmpeq_epi8(chunk, colon));
        let combined = _mm_or_si128(_mm_or_si128(m0, m1), m2);
        
        let mask = _mm_movemask_epi8(combined) as u16;
        
        if mask != 0 {
            return Some(offset + mask.trailing_zeros() as usize);
        }
        offset += 16;
    }

    data[offset..].iter().position(|&b| is_structural(b)).map(|p| offset + p)
}

#[cfg(not(target_arch = "x86_64"))]
pub fn find_structural_simd(data: &[u8]) -> Option<usize> {
    data.iter().position(|&b| is_structural(b))
}

#[inline]
pub fn find_structural(data: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    { if is_x86_feature_detected!("sse2") { unsafe { find_structural_simd(data) } } else { find_structural_scalar(data) } }
    #[cfg(not(target_arch = "x86_64"))]
    { find_structural_scalar(data) }
}

pub fn find_structural_scalar(data: &[u8]) -> Option<usize> {
    data.iter().position(|&b| is_structural(b))
}