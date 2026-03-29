//! SIMD-accelerated JSON primitives.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Skip whitespace using AVX2 (32 bytes at a time).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn skip_whitespace_avx2(data: &[u8]) -> usize {
    if data.len() < 32 { return skip_whitespace_scalar(data); }

    let first = *data.as_ptr();
    if first != b' ' && first != b'\t' && first != b'\n' && first != b'\r' {
        return 0;
    }

    let spaces = _mm256_set1_epi8(32_i8);
    let nine = _mm256_set1_epi8(9_i8);
    let five = _mm256_set1_epi8(5_i8);

    let mut offset = 0;
    let end = data.len();

    while offset + 32 <= end {
        let chunk = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        
        let eq_space = _mm256_cmpeq_epi8(chunk, spaces);
        let shifted = _mm256_sub_epi8(chunk, nine);
        let in_range = _mm256_cmpgt_epi8(five, shifted);
        
        let ws = _mm256_or_si256(eq_space, in_range);
        let mask = _mm256_movemask_epi8(ws) as u32;

        if mask == 0xffffffff {
            offset += 32;
        } else {
            return offset + (!mask).trailing_zeros() as usize;
        }
    }

    offset + skip_whitespace_scalar(&data[offset..])
}

pub fn skip_whitespace_scalar(data: &[u8]) -> usize {
    if data.is_empty() { return 0; }
    let first = unsafe { *data.as_ptr() };
    if first != b' ' && first != b'\t' && first != b'\n' && first != b'\r' {
        return 0;
    }
    
    let ptr = data.as_ptr();
    let len = data.len();
    let mut i = 0;
    
    while i < len {
        let b = unsafe { *ptr.add(i) };
        if b != b' ' && b != b'\t' && b != b'\n' && b != b'\r' {
            return i;
        }
        i += 1;
    }
    len
}

/// Find string end using AVX2 - optimized for unescaped strings.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn find_string_end_avx2(data: &[u8]) -> Option<(usize, bool)> {
    if data.is_empty() { return None; }

    let quotes = _mm256_set1_epi8(b'"' as i8);
    let backslashes = _mm256_set1_epi8(b'\\' as i8);

    let mut offset = 0;
    let end = data.len();

    // Fast path: scan for quote without escape tracking
    while offset + 32 <= end {
        let chunk = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let quote_mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, quotes)) as u32;
        let backslash_mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, backslashes)) as u32;
        
        // No backslashes: if we found a quote, return immediately (no escapes)
        if backslash_mask == 0 && quote_mask != 0 {
            let pos = offset + quote_mask.trailing_zeros() as usize;
            return Some((pos, false));
        }
        
        // No quotes: continue scanning
        if quote_mask == 0 {
            offset += 32;
            continue;
        }
        
        // Found both quote and backslash in same chunk - need byte-by-byte
        let mut escaped = false;
        let mut i = 0;
        while i < 32 && offset + i < end {
            let b = *data.as_ptr().add(offset + i);
            if b == b'\\' { escaped = !escaped; }
            else if b == b'"' && !escaped { return Some((offset + i, true)); }
            else { escaped = false; }
            i += 1;
        }
        offset += 32;
        
        // Remaining bytes must be scalar with escape tracking
        let mut escaped = false;
        let has_escapes = true;
        for i in offset..end {
            let b = *data.as_ptr().add(i);
            if b == b'\\' { escaped = !escaped; }
            else if b == b'"' && !escaped { return Some((i, has_escapes)); }
            else { escaped = false; }
        }
        return None;
    }

    // Tail
    let mut escaped = false;
    let mut has_escapes = false;
    for i in offset..end {
        let b = *data.as_ptr().add(i);
        if b == b'\\' { escaped = !escaped; has_escapes = true; }
        else if b == b'"' && !escaped { return Some((i, has_escapes)); }
        else { escaped = false; }
    }
    None
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

#[inline(always)]
pub fn skip_whitespace(data: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if data.len() < 32 { return skip_whitespace_scalar(data); }
        let first = unsafe { *data.as_ptr() };
        if first != b' ' && first != b'\t' && first != b'\n' && first != b'\r' {
            return 0;
        }
        unsafe { skip_whitespace_avx2(data) }
    }
    #[cfg(not(target_arch = "x86_64"))]
    { skip_whitespace_scalar(data) }
}

#[inline(always)]
pub fn find_string_end(data: &[u8]) -> Option<(usize, bool)> {
    #[cfg(target_arch = "x86_64")]
    {
        if data.len() < 32 { return find_string_end_scalar(data); }
        unsafe { find_string_end_avx2(data) }
    }
    #[cfg(not(target_arch = "x86_64"))]
    { find_string_end_scalar(data) }
}
