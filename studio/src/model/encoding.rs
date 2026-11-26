//! Base64 encoding (the standard alphabet, with `=` padding).
//!
//! Two places need it: the clipboard copy in `app` (the terminal's OSC 52
//! sequence) and the download check in `system/download.rs` (a `sha256-` hash in
//! Subresource Integrity form). It is a small leaf module, so neither of them
//! depends on the other for it.

/// Encodes `data` as standard base64 with `=` padding.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(base64(b"abc"), "YWJj");
/// ```
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b"#9f6434"), "IzlmNjQzNA==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
    }
}
