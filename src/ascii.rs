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

#[cfg(all(test, feature = "nightly"))]
mod benches {
    extern crate test;

    use super::*;

    #[bench]
    fn bench_no_escape_small(b: &mut test::Bencher) {
        let s = "abc";
        b.iter(|| escape('\\', s, [b'"']));
    }

    #[bench]
    fn bench_no_escape_medium(b: &mut test::Bencher) {
        let s = "abcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape('\\', s, [b'"']));
    }

    #[bench]
    fn bench_no_escape_large(b: &mut test::Bencher) {
        let s = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape('\\', s, [b'"']));
    }

    #[bench]
    fn bench_escape_small(b: &mut test::Bencher) {
        let s = "a\\b";
        b.iter(|| escape('\\', s, [b'"']));
    }

    #[bench]
    fn bench_escape_medium(b: &mut test::Bencher) {
        let s = "a\\bcdefgh\\ijklmnopqrst\\uvwxyz";
        b.iter(|| escape('\\', s, [b'"']));
    }

    #[bench]
    fn bench_escape_large(b: &mut test::Bencher) {
        let s = "a\\bcdefgh\\ijklmnopqrst\\uvwxyzabcdefghijklmnopqrstuvwxyz\\abcdefghijklmnopqrstuvwxyz\\abcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape('\\', s, [b'"']));
    }
}
