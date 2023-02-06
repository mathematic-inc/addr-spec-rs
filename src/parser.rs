use std::{fmt, mem::ManuallyDrop, str::Chars};

use super::unicode;
use super::AddrSpec;

pub const fn is_ascii_control_and_not_htab(chr: char) -> bool {
    chr.is_ascii_control() && chr != '\t'
}

pub const fn is_ascii_control_or_space(chr: char) -> bool {
    chr.is_ascii_control() || chr == ' '
}

pub const fn is_not_atext(chr: char) -> bool {
    is_ascii_control_or_space(chr)
        || matches!(
            chr,
            '"' | '(' | ')' | ',' | ':' | '<' | '>' | '@' | '[' | ']' | '\\'
        )
}

pub const fn is_not_dtext(chr: char) -> bool {
    is_ascii_control_or_space(chr) || matches!(chr, '[' | ']' | '\\')
}

/// A error that can occur when parsing or creating an address specification.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ParseError(pub(super) &'static str, pub(super) usize);

impl ParseError {
    /// Returns a static error message.
    #[inline]
    pub fn message(&self) -> &'static str {
        self.0
    }

    /// Returns the byte index where the error occurred.
    #[inline]
    pub fn index(&self) -> usize {
        self.1
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "parse error at index {}: {}",
            self.message(),
            self.index()
        )
    }
}

pub struct Parser<'a> {
    input: &'a str,
    iterator: Chars<'a>,
}

impl<'a> Parser<'a> {
    #[inline]
    pub fn new(input: &'a str) -> Parser<'a> {
        Parser {
            input,
            iterator: input.chars(),
        }
    }

    pub fn parse(mut self) -> Result<AddrSpec, ParseError> {
        #[cfg(feature = "white-spaces")]
        self.parse_cfws()?;
        let local_part = self.parse_local_part()?;
        #[cfg(feature = "white-spaces")]
        self.parse_cfws()?;
        self.skip_at()?;
        #[cfg(feature = "white-spaces")]
        self.parse_cfws()?;
        // `literal` only used when feature is enabled
        #[allow(unused_variables)]
        let (domain, literal) = self.parse_domain()?;
        #[cfg(feature = "white-spaces")]
        self.parse_cfws()?;
        self.check_end("expected end of address")?;
        Ok(AddrSpec {
            local_part,
            domain,
            #[cfg(feature = "literals")]
            literal,
        })
    }

    #[cfg(feature = "white-spaces")]
    fn parse_cfws(&mut self) -> Result<(), ParseError> {
        self.skip_fws();
        #[cfg(feature = "comments")]
        while self.eat_chr('(') {
            self.parse_comment()?;
            self.skip_fws();
        }
        Ok(())
    }

    #[cfg(feature = "white-spaces")]
    fn skip_fws(&mut self) {
        self.skip_ws();
        if !self.eat_str("\r\n") {
            return;
        }
        self.skip_ws();
    }

    #[cfg(feature = "white-spaces")]
    fn skip_ws(&mut self) {
        loop {
            if !self.eat_slice([' ', '\t']) {
                break;
            }
        }
    }

    #[cfg(feature = "white-spaces")]
    fn eat_slice<const N: usize>(&mut self, pattern: [char; N]) -> bool {
        if self.iterator.as_str().starts_with(pattern) {
            self.iterator.next();
            return true;
        }
        false
    }

    #[cfg(feature = "white-spaces")]
    fn eat_str(&mut self, pattern: &str) -> bool {
        if let Some(input) = self.iterator.as_str().strip_prefix(pattern) {
            self.iterator = input.chars();
            return true;
        }
        false
    }

    fn eat_chr(&mut self, pattern: char) -> bool {
        if self.iterator.as_str().starts_with(pattern) {
            self.iterator.next();
            return true;
        }
        false
    }

    #[cfg(feature = "comments")]
    fn parse_comment(&mut self) -> Result<(), ParseError> {
        #[cfg(feature = "white-spaces")]
        self.skip_fws();

        let mut nest_level = 1usize;
        while let Some(chr) = self.iterator.next() {
            match chr {
                ')' => {
                    if nest_level == 1 {
                        return Ok(());
                    }
                    nest_level -= 1;
                }
                '\\' => {
                    self.parse_quoted_pair()?;
                }
                '(' => {
                    nest_level += 1;
                }
                chr => {
                    if is_ascii_control_or_space(chr) {
                        return Err(self.error("invalid character in comment", -1));
                    }
                }
            }

            #[cfg(feature = "white-spaces")]
            self.skip_fws();
        }

        Err(self.error("expected ')' for comment", 0))
    }

    fn parse_quoted_pair(&mut self) -> Result<char, ParseError> {
        match self.iterator.next() {
            Some(chr) if !is_ascii_control_and_not_htab(chr) => Ok(chr),
            Some(_) => Err(self.error("invalid character in quoted pair", -1)),
            None => Err(self.error("unexpected end of quoted pair", 0)),
        }
    }

    fn parse_local_part(&mut self) -> Result<String, ParseError> {
        if !self.eat_chr('"') {
            return Ok(unicode::normalize(
                self.parse_dot_atom("empty label in local part")?,
            ));
        }
        Ok(unicode::normalize(self.parse_quoted_string(
            "invalid character in quoted local part",
            "expected '\"' for quoted local part",
        )?))
    }

    pub fn parse_dot_atom(
        &mut self,
        empty_label_error_text: &'static str,
    ) -> Result<&str, ParseError> {
        let input = self.iterator.as_str();
        let size = input.find(is_not_atext).unwrap_or(input.len());

        let dot_atom = &input[..size];
        if let Some(offset) = dot_atom
            .split('.')
            .find(|label| label.is_empty())
            .map(|label| label.as_ptr() as usize - dot_atom.as_ptr() as usize)
        {
            return Err(self.error(empty_label_error_text, offset as isize));
        }

        self.iterator = input[size..].chars();
        Ok(dot_atom)
    }

    fn parse_quoted_string(
        &mut self,
        invalid_character_error_text: &'static str,
        expected_quote_error_text: &'static str,
    ) -> Result<String, ParseError> {
        #[cfg(feature = "white-spaces")]
        self.skip_fws();

        let mut quoted_string = unsafe { FixedVec::new(self.iterator.as_str().len()) };
        while let Some(chr) = self.iterator.next() {
            let chr = match chr {
                '"' => return Ok(quoted_string.into()),
                '\\' => self.parse_quoted_pair()?,
                chr if is_ascii_control_or_space(chr) => {
                    return Err(self.error(invalid_character_error_text, -1))
                }
                chr => chr,
            };
            unsafe {
                quoted_string.extend_char_unchecked(chr);
            }

            #[cfg(feature = "white-spaces")]
            self.skip_fws();
        }

        Err(self.error(expected_quote_error_text, 0))
    }

    fn skip_at(&mut self) -> Result<(), ParseError> {
        if self.eat_chr('@') {
            return Ok(());
        }
        Err(self.error("expected '@'", 1))
    }

    fn parse_domain(&mut self) -> Result<(String, bool), ParseError> {
        #[cfg(feature = "literals")]
        if self.eat_chr('[') {
            return Ok((unicode::normalize(self.parse_domain_literal()?), true));
        }
        Ok((
            unicode::normalize(self.parse_dot_atom("empty label in domain")?),
            false,
        ))
    }

    #[cfg(all(feature = "literals", not(feature = "white-spaces")))]
    fn parse_domain_literal(&mut self) -> Result<&str, ParseError> {
        let input = self.iterator.as_str();
        let size = input.find(is_not_dtext).unwrap_or(input.len());

        self.iterator = input[size..].chars();
        if !self.eat_chr(']') {
            return Err(self.error("expected ']' for domain literal", 0));
        }

        Ok(&input[..size])
    }

    #[cfg(all(feature = "literals", feature = "white-spaces"))]
    fn parse_domain_literal(&mut self) -> Result<String, ParseError> {
        #[cfg(feature = "white-spaces")]
        self.skip_fws();

        let mut domain = unsafe { FixedVec::new(self.iterator.as_str().len()) };
        while let Some(chr) = self.iterator.next() {
            let chr = match chr {
                ']' => return Ok(domain.into()),
                chr if is_not_dtext(chr) => {
                    return Err(self.error("invalid character in literal domain", -1))
                }
                chr => chr,
            };
            unsafe {
                domain.extend_char_unchecked(chr);
            }

            #[cfg(feature = "white-spaces")]
            self.skip_fws();
        }

        Err(self.error("expected ']' for domain literal", 0))
    }

    #[inline]
    pub fn check_end(self, message: &'static str) -> Result<(), ParseError> {
        if self.iterator.as_str().is_empty() {
            return Ok(());
        }
        Err(self.error(message, 0))
    }

    fn error(&self, message: &'static str, offset: isize) -> ParseError {
        ParseError(
            message,
            (self.input.len() - self.iterator.as_str().len())
                .checked_add_signed(offset)
                .unwrap(),
        )
    }
}

pub struct FixedVec<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
}

impl<T> FixedVec<T> {
    pub unsafe fn new(cap: usize) -> Self {
        Self {
            ptr: unsafe { std::alloc::alloc(std::alloc::Layout::array::<T>(cap).unwrap()).cast() },
            len: 0,
            cap,
        }
    }

    unsafe fn extend_unchecked(&mut self, slice: &[T]) {
        unsafe {
            std::ptr::copy_nonoverlapping(slice.as_ptr(), self.ptr.add(self.len), slice.len());
        }
        self.len += slice.len();
        debug_assert!(self.len <= self.cap);
    }
}

impl FixedVec<u8> {
    unsafe fn extend_char_unchecked(&mut self, chr: char) {
        self.extend_unchecked(chr.encode_utf8(&mut [0; 4]).as_bytes())
    }
}

impl<T> Drop for FixedVec<T> {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.ptr.cast(),
                std::alloc::Layout::array::<T>(self.cap).unwrap(),
            )
        }
    }
}

impl From<FixedVec<u8>> for String {
    fn from(val: FixedVec<u8>) -> Self {
        let val = ManuallyDrop::new(val);
        unsafe { String::from_raw_parts(val.ptr, val.len, val.cap) }
    }
}

#[cfg(test)]
mod tests {
    mod dot_atoms {
        use super::super::{ParseError, Parser};

        #[test]
        fn test_parse_local_part() {
            assert_eq!(&Parser::new("test").parse_local_part().unwrap(), "test")
        }

        #[test]
        fn test_parse_empty_local_part() {
            assert_eq!(
                Parser::new("").parse_local_part().unwrap_err(),
                ParseError("empty label in local part", 0)
            )
        }

        #[test]
        fn test_parse_local_part_with_empty_label_in_front() {
            assert_eq!(
                Parser::new(".test").parse_local_part().unwrap_err(),
                ParseError("empty label in local part", 0)
            )
        }

        #[test]
        fn test_parse_local_part_with_empty_label_in_middle() {
            assert_eq!(
                Parser::new("te..st").parse_local_part().unwrap_err(),
                ParseError("empty label in local part", 3)
            )
        }

        #[test]
        fn test_parse_local_part_with_empty_label_in_back() {
            assert_eq!(
                Parser::new("test.").parse_local_part().unwrap_err(),
                ParseError("empty label in local part", 5)
            )
        }

        #[test]
        fn test_parse_domain() {
            assert_eq!(
                Parser::new("test").parse_domain().unwrap(),
                ("test".to_string(), false)
            )
        }

        #[test]
        fn test_parse_empty_domain() {
            assert_eq!(
                Parser::new("").parse_domain().unwrap_err(),
                ParseError("empty label in domain", 0)
            )
        }

        #[test]
        fn test_parse_domain_with_empty_label_in_front() {
            assert_eq!(
                Parser::new(".test").parse_domain().unwrap_err(),
                ParseError("empty label in domain", 0)
            )
        }

        #[test]
        fn test_parse_domain_with_empty_label_in_middle() {
            assert_eq!(
                Parser::new("te..st").parse_domain().unwrap_err(),
                ParseError("empty label in domain", 3)
            )
        }

        #[test]
        fn test_parse_domain_with_empty_label_in_back() {
            assert_eq!(
                Parser::new("test.").parse_domain().unwrap_err(),
                ParseError("empty label in domain", 5)
            )
        }
    }

    #[cfg(feature = "literals")]
    mod literals {
        use super::super::{ParseError, Parser};

        #[test]
        fn test_parse_literal_domain() {
            assert_eq!(
                Parser::new("[test]").parse_domain().unwrap(),
                ("test".to_string(), true)
            )
        }

        #[test]
        fn test_parse_literal_domain_without_bracket() {
            assert_eq!(
                Parser::new("[test").parse_domain().unwrap_err(),
                ParseError("expected ']' for domain literal", 5)
            )
        }

        #[test]
        fn test_parse_empty_literal_domain() {
            assert_eq!(
                Parser::new("[]").parse_domain().unwrap(),
                ("".to_string(), true)
            )
        }

        #[test]
        fn test_parse_empty_literal_domain_without_bracket() {
            assert_eq!(
                Parser::new("[").parse_domain().unwrap_err(),
                ParseError("expected ']' for domain literal", 1)
            )
        }

        #[cfg(not(feature = "white-spaces"))]
        #[test]
        fn test_parse_literal_domain_with_white_spaces() {
            assert_eq!(
                Parser::new("[te st]").parse_domain().unwrap_err(),
                ParseError("expected ']' for domain literal", 3)
            )
        }

        #[cfg(feature = "white-spaces")]
        #[test]
        fn test_parse_literal_domain_with_white_spaces() {
            assert_eq!(
                Parser::new("[te st]").parse_domain().unwrap(),
                ("test".to_string(), true)
            )
        }

        #[cfg(feature = "white-spaces")]
        #[test]
        fn test_parse_literal_domain_with_fws_in_front() {
            assert_eq!(
                Parser::new("[\r\ntest]").parse_domain().unwrap(),
                ("test".to_string(), true)
            )
        }

        #[cfg(feature = "white-spaces")]
        #[test]
        fn test_parse_literal_domain_with_fws_in_middle() {
            assert_eq!(
                Parser::new("[te\r\nst]").parse_domain().unwrap(),
                ("test".to_string(), true)
            )
        }

        #[cfg(feature = "white-spaces")]
        #[test]
        fn test_parse_literal_domain_with_fws_in_back() {
            assert_eq!(
                Parser::new("[test\r\n]").parse_domain().unwrap(),
                ("test".to_string(), true)
            )
        }
    }
}
