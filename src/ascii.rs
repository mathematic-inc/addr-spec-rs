use std::alloc::{alloc, Layout};
use std::os::raw::c_char;

include!(concat!(env!("OUT_DIR"), "/ascii.rs"));

pub fn escape<const N: usize>(esc_chr: char, src: &str, cntl_chrs: [u8; N]) -> String {
    let cap = src.len() << 1;
    unsafe {
        let dst = alloc(Layout::array::<u8>(cap).unwrap());
        let len = ascii_escape(
            esc_chr as c_char,
            src.as_ptr() as *const c_char,
            src.len(),
            dst as *mut c_char,
            cntl_chrs.as_ptr() as *const c_char,
            N,
        );
        String::from_raw_parts(dst, len, cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape() {
        assert_eq!(escape('\\', "", [b'"']), "");
        assert_eq!(escape('\\', "abc", [b'"']), "abc");
        assert_eq!(escape('\\', "a\\b", [b'"']), "a\\\\b");
        assert_eq!(escape('\\', "a\"b", [b'"']), "a\\\"b");
        assert_eq!(escape('\\', "a\\\"b", [b'"']), "a\\\\\\\"b");
        assert_eq!(escape('\\', "😄\"😄😄", [b'"']), "😄\\\"😄😄");
    }
}
