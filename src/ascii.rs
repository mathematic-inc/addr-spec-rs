macro_rules! memcpsn {
    ($value:ident, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        {
            let mut i = 0;
            loop {
                if i == $value.len() {
                    break None;
                }
                if matches!($value[i], $( $pattern )|+ $( if $guard )?) {
                    break Some(i);
                }
                i += 1;
            }
        }
    };
}

pub(crate) use memcpsn;

macro_rules! escape {
    ($expression:expr, $escape_char:literal, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        {
            let cap = $expression.len() << 1;
            unsafe {
                let buffer = std::alloc::alloc(std::alloc::Layout::array::<u8>(cap).unwrap());
                let mut src = $expression.as_bytes();
                let mut dst = buffer;
                while let Some(end) = $crate::ascii::memcpsn!(src, $escape_char | $( $pattern )|+ $( if $guard )?) {
                    dst.copy_from_nonoverlapping(src.as_ptr(), end);
                    dst = dst.add(end);
                    dst.copy_from_nonoverlapping([$escape_char, src[end]].as_ptr(), 2);
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
    };
}

pub(crate) use escape;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape() {
        assert_eq!(escape!("", b'\\', b'"'), "");
        assert_eq!(escape!("abc", b'\\', b'"'), "abc");
        assert_eq!(escape!("a\\b", b'\\', b'"'), "a\\\\b");
        assert_eq!(escape!("a\"b", b'\\', b'"'), "a\\\"b");
        assert_eq!(escape!("a\\\"b", b'\\', b'"'), "a\\\\\\\"b");
        assert_eq!(escape!("😄\"😄😄", b'\\', b'"'), "😄\\\"😄😄");
        assert_eq!(escape!("😄😄😄\"", b'\\', b'"'), "😄😄😄\\\"");
    }
}

#[cfg(all(test, feature = "nightly"))]
mod benches {
    extern crate test;

    use super::*;

    #[bench]
    fn bench_no_escape_small(b: &mut test::Bencher) {
        let s = "abc";
        b.iter(|| escape!(s, b'\\', b'"'));
    }

    #[bench]
    fn bench_no_escape_medium(b: &mut test::Bencher) {
        let s = "abcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape!(s, b'\\', b'"'));
    }

    #[bench]
    fn bench_no_escape_large(b: &mut test::Bencher) {
        let s = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape!(s, b'\\', b'"'));
    }

    #[bench]
    fn bench_escape_small(b: &mut test::Bencher) {
        let s = "a\\b";
        b.iter(|| escape!(s, b'\\', b'"'));
    }

    #[bench]
    fn bench_escape_medium(b: &mut test::Bencher) {
        let s = "a\\bcdefgh\\ijklmnopqrst\\uvwxyz";
        b.iter(|| escape!(s, b'\\', b'"'));
    }

    #[bench]
    fn bench_escape_large(b: &mut test::Bencher) {
        let s = "a\\bcdefgh\\ijklmnopqrst\\uvwxyzabcdefghijklmnopqrstuvwxyz\\abcdefghijklmnopqrstuvwxyz\\abcdefghijklmnopqrstuvwxyz";
        b.iter(|| escape!(s, b'\\', b'"'));
    }

    #[bench]
    fn bench_many_escapes(b: &mut test::Bencher) {
        let s = "\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"\\\"";
        b.iter(|| escape!(s, b'\\', b'"'));
    }
}
