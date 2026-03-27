//! SIMD-accelerated JSON primitives.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Skip whitespace using SIMD (16 bytes at a time).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub unsafe fn skip_whitespace_simd(data: &[u8]) -> usize {
    if data.len() < 16 { return skip_whitespace_scalar(data); }

    // Fast check: if first byte is not whitespace, return immediately
    let first = data[0];
    if first != b' ' && first != b'\t' && first != b'\n' && first != b'\r' {
        return 0;
    }

    // Create lookup: whitespace chars (space=32, tab=9, newline=10, cr=13)
    // Use a single comparison trick: for ws chars, (c - 1) < 14 where bits 9,10,13 are set
    // Actually simpler: use signed comparison trick
    // ws if c == 32 OR (c >= 9 AND c <= 13)
    // Optimized: create mask where bit c is set for ws chars
    let ws_mask = _mm_setr_epi8(
        0, 0, 0, 0, 0, 0, 0, 0,  // 0-7: not ws
        0, 1, 1, 1, 1, 1, 0, 0   // 8-15: 9,10,11,12,13 are ws
    );
    
    let spaces = _mm_set1_epi8(32);  // space character

    let mut offset = 0;
    let chunks = data.len() / 16;

    for _ in 0..chunks {
        let chunk = _mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i);
        
        // Check for space (32)
        let eq_space = _mm_cmpeq_epi8(chunk, spaces);
        
        // Check for 9-13: subtract 9, check if < 5 using unsigned comparison
        let shifted = _mm_sub_epi8(chunk, _mm_set1_epi8(9));
        let in_range = _mm_cmplt_epi8(shifted, _mm_set1_epi8(5));
        
        let ws = _mm_or_si128(eq_space, in_range);
        let mask = _mm_movemask_epi8(ws) as u32;

        if mask == 0xffff {
            offset += 16;
        } else {
            return offset + (!mask).trailing_zeros() as usize;
        }
    }
    offset + skip_whitespace_scalar(&data[offset..])
}

#[cfg(not(target_arch = "x86_64"))]
pub fn skip_whitespace_simd(data: &[u8]) -> usize {
    skip_whitespace_scalar(data)
}

pub fn skip_whitespace_scalar(data: &[u8]) -> usize {
    // Pointer-based iteration for maximum speed
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

// Inline the feature check - compiler can optimize this better
#[inline(always)]
pub fn skip_whitespace(data: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if data.len() < 16 { return skip_whitespace_scalar(data); }
        
        // Fast check: if first byte is not whitespace, return immediately
        // This avoids SIMD overhead for the common case of no whitespace
        let first = unsafe { *data.as_ptr() };
        if first != b' ' && first != b'\t' && first != b'\n' && first != b'\r' {
            return 0;
        }
        
        unsafe { skip_whitespace_simd(data) }
    }
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
    let mut has_escapes = false;

    // Fast path: first scan for quote and backslash in parallel
    while offset + 16 <= data.len() {
        let chunk = _mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i);
        let quote_mask = _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, quotes)) as u16;
        let backslash_mask = _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, backslashes)) as u16;
        
        // If no escape in this chunk and we found a quote, return immediately
        if backslash_mask == 0 && quote_mask != 0 {
            let pos = offset + quote_mask.trailing_zeros() as usize;
            return Some((pos, has_escapes));
        }
        
        // If we have escapes, mark it
        if backslash_mask != 0 { has_escapes = true; }
        
        // If no quotes and no escapes, continue
        if quote_mask == 0 {
            offset += 16;
            continue;
        }
        
        // Complex case: both quotes and escapes in this chunk
        // We need to track escape state
        let mut escaped = false;
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

    // Handle remaining bytes
    let mut escaped = false;
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
    { unsafe { find_string_end_simd(data) } }
    #[cfg(not(target_arch = "x86_64"))]
    { find_string_end_scalar(data) }
}

// Note: structural character finding removed - not used in current implementation