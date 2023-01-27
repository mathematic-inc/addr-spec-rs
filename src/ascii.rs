use std::alloc::{alloc, Layout};

fn memcspn(value: &[u8], accept: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < value.len() {
        if memchr::memchr(value[i], accept).is_some() {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Escape ASCII characters in the given string. The first character in `cntl_chrs`
/// is used as the escape character.
pub fn escape<const N: usize>(value: &str, escape: [u8; N]) -> String {
    let cap = value.len() << 1;
    unsafe {
        let buffer = alloc(Layout::array::<u8>(cap).unwrap());
        let mut src = value.as_bytes();
        let mut dst = buffer;
        while let Some(end) = memcspn(src, &escape) {
            dst.copy_from_nonoverlapping(src.as_ptr(), end);
            dst = dst.add(end);
            dst.copy_from_nonoverlapping([escape[0], src[end]].as_ptr(), 2);
            dst = dst.add(2);
            src = &src[end + 1..];
        }
        if !src.is_empty() {
            dst.copy_from_nonoverlapping(src.as_ptr(), src.len());
            dst = dst.add(src.len());
        }
        String::from_raw_parts(buffer, dst.offset_from(buffer) as usize, cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape() {
        assert_eq!(escape("", [b'\\', b'"']), "");
        assert_eq!(escape("abc", [b'\\', b'"']), "abc");
        assert_eq!(escape("a\\b", [b'\\', b'"']), "a\\\\b");
        assert_eq!(escape("a\"b", [b'\\', b'"']), "a\\\"b");
        assert_eq!(escape("a\\\"b", [b'\\', b'"']), "a\\\\\\\"b");
        assert_eq!(escape("😄\"😄😄", [b'\\', b'"']), "😄\\\"😄😄");
        assert_eq!(escape("😄😄😄\"", [b'\\', b'"']), "😄😄😄\\\"");
    }
}

#[cfg(all(test, feature = "nightly"))]
mod benches {
    extern crate test;

    use super::*;

    #[bench]
    fn bench_no_escape_small(b: &mut test::Bencher) {
        let s = "abc";
        b.iter(|| escape(s, [b'\\', b'"']));
    }

    #[bench]
    fn bench_no_escape_medium(b: &mut test::Bencher) {
        let s = "abcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape(s, [b'\\', b'"']));
    }

    #[bench]
    fn bench_no_escape_large(b: &mut test::Bencher) {
        let s = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape(s, [b'\\', b'"']));
    }

    #[bench]
    fn bench_escape_small(b: &mut test::Bencher) {
        let s = "a\\b";
        b.iter(|| escape(s, [b'\\', b'"']));
    }

    #[bench]
    fn bench_escape_medium(b: &mut test::Bencher) {
        let s = "a\\bcdefgh\\ijklmnopqrst\\uvwxyz";
        b.iter(|| escape(s, [b'\\', b'"']));
    }

    #[bench]
    fn bench_escape_large(b: &mut test::Bencher) {
        let s = "a\\bcdefgh\\ijklmnopqrst\\uvwxyzabcdefghijklmnopqrstuvwxyz\\abcdefghijklmnopqrstuvwxyz\\abcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape(s, [b'\\', b'"']));
    }

    #[bench]
    fn bench_many_escapes(b: &mut test::Bencher) {
        let s = "\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"";
        b.iter(|| escape(s, [b'\\', b'"']));
    }
}
