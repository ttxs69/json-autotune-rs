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
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub unsafe fn find_string_end_simd(data: &[u8]) -> Option<usize> {
    if data.is_empty() { return None; }

    let quotes = _mm_set1_epi8(b'"' as i8);
    let backslashes = _mm_set1_epi8(b'\\' as i8);

    let mut offset = 0;
    let mut escaped = false;

    while offset + 16 <= data.len() {
        let chunk = _mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i);
        let quote_mask = _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, quotes)) as u16;
        let backslash_mask = _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, backslashes)) as u16;

        for i in 0..16 {
            if offset + i >= data.len() { return None; }
            if (backslash_mask >> i) & 1 != 0 { escaped = !escaped; }
            else if (quote_mask >> i) & 1 != 0 {
                if !escaped { return Some(offset + i); }
                escaped = false;
            } else { escaped = false; }
        }
        offset += 16;
    }

    // Scalar fallback for remaining
    for i in offset..data.len() {
        if data[i] == b'\\' { escaped = !escaped; }
        else if data[i] == b'"' && !escaped { return Some(i); }
        else { escaped = false; }
    }
    None
}

#[cfg(not(target_arch = "x86_64"))]
pub fn find_string_end_simd(data: &[u8]) -> Option<usize> {
    find_string_end_scalar(data)
}

pub fn find_string_end_scalar(data: &[u8]) -> Option<usize> {
    let mut escaped = false;
    for (i, &b) in data.iter().enumerate() {
        if b == b'\\' { escaped = !escaped; }
        else if b == b'"' && !escaped { return Some(i); }
        else { escaped = false; }
    }
    None
}

pub fn find_string_end(data: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    { if is_x86_feature_detected!("sse2") { unsafe { find_string_end_simd(data) } } else { find_string_end_scalar(data) } }
    #[cfg(not(target_arch = "x86_64"))]
    { find_string_end_scalar(data) }
}