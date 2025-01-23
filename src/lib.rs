#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "nightly", feature(test))]

mod ascii;
mod parser;
mod unicode;

use std::{
    fmt::{self, Write},
    str::FromStr,
};

pub use parser::ParseError;
use parser::{is_ascii_control_and_not_htab, is_not_atext, is_not_dtext, Parser};

fn quote(value: &str) -> String {
    ascii::escape!(value, b'\\', b'"' | b' ' | b'\t')
}

/// Address specification as defined in [RFC
/// 5322](https://tools.ietf.org/html/rfc5322#section-3.4.1) with UTF-8 support
/// as defined in [RFC 6532](https://tools.ietf.org/html/rfc6532).
///
/// Both the local part and the domain are normalized using the
/// [NFC](https://unicode.org/reports/tr15/#Norm_Forms) as recommended in
/// [Section 3.1, RFC 6532](https://tools.ietf.org/html/rfc6532#section-3.1).
/// Address strings built using this crate work well for unique, UTF-8
/// identifiers.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
///
/// use addr_spec::AddrSpec;
///
/// let addr_spec = AddrSpec::from_str("test@example.com").unwrap();
/// assert_eq!(addr_spec.local_part(), "test");
/// assert_eq!(addr_spec.domain(), "example.com");
/// assert_eq!(addr_spec.is_literal(), false);
/// assert_eq!(addr_spec.to_string(), "test@example.com");
/// ```
///
/// Quoted local parts will be unescaped if possible:
///
/// ```
/// use std::str::FromStr;
///
/// use addr_spec::AddrSpec;
///
/// let addr_spec = AddrSpec::from_str(r#""test"@example.com"#).unwrap();
/// assert_eq!(addr_spec.local_part(), "test");
/// assert_eq!(addr_spec.domain(), "example.com");
/// assert_eq!(addr_spec.is_literal(), false);
/// assert_eq!(addr_spec.to_string(), "test@example.com");
/// ```
///
/// Literal domains are also supported:
///
/// ```
/// use std::str::FromStr;
///
/// use addr_spec::AddrSpec;
///
/// #[cfg(feature = "literals")]
/// {
///     let addr_spec = AddrSpec::from_str("test@[IPv6:2001:db8::1]").unwrap();
///     assert_eq!(addr_spec.local_part(), "test");
///     assert_eq!(addr_spec.domain(), "IPv6:2001:db8::1");
///     assert_eq!(addr_spec.is_literal(), true);
///     assert_eq!(addr_spec.to_string(), "test@[IPv6:2001:db8::1]");
/// }
/// ```
///
/// You can also create an address specification from its parts:
///
/// ```
/// use addr_spec::AddrSpec;
///
/// let addr_spec = AddrSpec::new("test", "example.com").unwrap();
/// assert_eq!(addr_spec.local_part(), "test");
/// assert_eq!(addr_spec.domain(), "example.com");
/// assert_eq!(addr_spec.is_literal(), false);
/// assert_eq!(addr_spec.to_string(), "test@example.com");
/// ```
///
/// If you want to just normalize an address, you can use the `normalize`
/// function:
///
/// ```
/// use addr_spec::AddrSpec;
///
/// assert_eq!(
///     &AddrSpec::normalize("\"test\"@example.com").unwrap(),
///     "test@example.com"
/// );
/// ```
///
/// # References
///
/// - [RFC 5322](https://tools.ietf.org/html/rfc5322#section-3.4.1)
/// - [RFC 6531](https://tools.ietf.org/html/rfc6531)
/// - [RFC 6532](https://tools.ietf.org/html/rfc6532)
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct AddrSpec {
    local_part: String,
    domain: String,
    #[cfg(feature = "literals")]
    literal: bool,
}

impl AddrSpec {
    /// Normalizes the address.
    ///
    /// This is a convenience function that parses the address and then
    /// serializes it again.
    ///
    /// It is equivalent to `address.parse::<AddrSpec>()?.to_string()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use addr_spec::AddrSpec;
    ///
    /// assert_eq!(
    ///     &AddrSpec::normalize("\"test\"@example.com").unwrap(),
    ///     "test@example.com"
    /// );
    /// ```
    #[inline]
    pub fn normalize<Address>(address: Address) -> Result<String, ParseError>
    where
        Address: AsRef<str>,
    {
        Ok(address.as_ref().parse::<Self>()?.to_string())
    }

    /// Creates a new address specification. This will validate the local part
    /// and domain and perform NFC-normalization.
    pub fn new<LocalPart, Domain>(local_part: LocalPart, domain: Domain) -> Result<Self, ParseError>
    where
        LocalPart: AsRef<str>,
        Domain: AsRef<str>,
    {
        Self::new_impl(local_part.as_ref(), domain.as_ref(), false)
    }

    /// Creates a new address specification with a literal domain. This will
    /// validate the local part and domain and perform NFC-normalization.
    #[cfg(feature = "literals")]
    pub fn with_literal<LocalPart, Domain>(
        local_part: LocalPart,
        domain: Domain,
    ) -> Result<Self, ParseError>
    where
        LocalPart: AsRef<str>,
        Domain: AsRef<str>,
    {
        Self::new_impl(local_part.as_ref(), domain.as_ref(), true)
    }

    fn new_impl(local_part: &str, domain: &str, literal: bool) -> Result<Self, ParseError> {
        if let Some(index) = local_part.find(is_ascii_control_and_not_htab) {
            return Err(ParseError("invalid character in local part", index));
        }

        if literal {
            if let Some(index) = domain.find(is_not_dtext) {
                return Err(ParseError("invalid character in literal domain", index));
            }
        } else {
            // We use the parser here since parsing dot atoms is a pure
            // operation (i.e. independent of any features).
            let mut parser = Parser::new(domain);
            parser.parse_dot_atom("empty label in domain")?;
            parser.check_end("invalid character in domain")?;
        }
        Ok(Self {
            local_part: unicode::normalize(local_part),
            domain: unicode::normalize(domain),
            #[cfg(feature = "literals")]
            literal,
        })
    }

    /// Creates a new address specification without performing any validation or
    /// normalization.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not validate nor normalize the
    /// local part or domain. If the local part or domain contains invalid
    /// characters or is not NFC-normalized, the resulting address specification
    /// will be invalid.
    ///
    /// Only use this function if you are sure that the local part and domain
    /// are valid and NFC-normalized. This is typically the case if you are
    /// getting them from a trusted source.
    #[inline]
    pub unsafe fn new_unchecked<LocalPart, Domain>(local_part: LocalPart, domain: Domain) -> Self
    where
        LocalPart: Into<String>,
        Domain: Into<String>,
    {
        Self::new_unchecked_impl(local_part.into(), domain.into(), false)
    }

    /// Creates a new address specification with a domain literal without
    /// performing any validation or normalization.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not validate nor normalize the
    /// local part or domain. If the local part or domain contains invalid
    /// characters or is not NFC-normalized, the resulting address specification
    /// will be invalid.
    ///
    /// Only use this function if you are sure that the local part and domain
    /// are valid and NFC-normalized. This is typically the case if you are
    /// getting them from a trusted source.
    #[cfg(feature = "literals")]
    #[inline]
    pub unsafe fn with_literal_unchecked<LocalPart, Domain>(
        local_part: LocalPart,
        domain: Domain,
    ) -> Self
    where
        LocalPart: Into<String>,
        Domain: Into<String>,
    {
        Self::new_unchecked_impl(local_part.into(), domain.into(), true)
    }

    #[allow(unused_variables)]
    unsafe fn new_unchecked_impl(local_part: String, domain: String, literal: bool) -> Self {
        Self {
            local_part,
            domain,
            #[cfg(feature = "literals")]
            literal,
        }
    }

    /// Returns the local part of the address.
    #[inline]
    pub fn local_part(&self) -> &str {
        &self.local_part
    }

    /// Returns the domain of the address.
    #[inline]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Returns whether the local part is quoted.
    #[inline]
    pub fn is_quoted(&self) -> bool {
        self.local_part()
            .split('.')
            .any(|s| s.is_empty() || s.contains(is_not_atext))
    }

    /// Returns whether the domain is literal.
    #[inline]
    pub fn is_literal(&self) -> bool {
        #[cfg(feature = "literals")]
        return self.literal;
        #[cfg(not(feature = "literals"))]
        return false;
    }

    /// Returns the local part and domain of the address.
    #[inline]
    pub fn into_parts(self) -> (String, String) {
        (self.local_part, self.domain)
    }

    /// Returns serialized versions of the local part and domain of the address.
    ///
    /// This is useful if you need to transport the address specification over
    /// line-based protocols such as SMTP and need to ensure that the local part
    /// and domain fit on a single line or require folding white-spaces.
    pub fn into_serialized_parts(self) -> (String, String) {
        // Note literals will be optimized away by the compiler if the feature
        // is disabled.
        match (self.is_quoted(), self.is_literal()) {
            (false, false) => (self.local_part, self.domain),
            (true, false) => (
                ["\"", &quote(self.local_part()), "\""].concat(),
                self.domain,
            ),
            (false, true) => (self.local_part, ["[", &self.domain, "]"].concat()),
            (true, true) => (
                ["\"", &quote(self.local_part()), "\""].concat(),
                ["[", &self.domain, "]"].concat(),
            ),
        }
    }
}

impl fmt::Display for AddrSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_quoted() {
            formatter.write_str(self.local_part())?;
        } else {
            formatter.write_char('"')?;
            for chr in quote(self.local_part()).chars() {
                formatter.write_char(chr)?;
            }
            formatter.write_char('"')?;
        }

        formatter.write_char('@')?;

        // Note literals will be optimized away by the compiler if the feature
        // is disabled.
        if !self.is_literal() {
            formatter.write_str(self.domain())?;
        } else {
            formatter.write_char('[')?;
            for chr in self.domain().chars() {
                formatter.write_char(chr)?;
            }
            formatter.write_char(']')?;
        }

        Ok(())
    }
}

impl FromStr for AddrSpec {
    type Err = ParseError;

    #[inline]
    fn from_str(address: &str) -> Result<Self, Self::Err> {
        Parser::new(address).parse()
    }
}

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "serde")]
impl Serialize for AddrSpec {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for AddrSpec {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "email_address")]
use email_address::EmailAddress;

#[cfg(feature = "email_address")]
impl From<EmailAddress> for AddrSpec {
    #[inline]
    fn from(val: EmailAddress) -> Self {
        AddrSpec::from_str(val.as_str()).unwrap()
    }
}

#[cfg(feature = "email_address")]
impl From<AddrSpec> for EmailAddress {
    #[inline]
    fn from(val: AddrSpec) -> Self {
        EmailAddress::new_unchecked(val.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addr_spec_from_str() {
        let addr_spec = AddrSpec::from_str("jdoe@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "white-spaces")]
    #[test]
    fn test_addr_spec_from_str_with_white_space_before_local_part() {
        let addr_spec = AddrSpec::from_str(" jdoe@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "white-spaces")]
    #[test]
    fn test_addr_spec_from_str_with_white_space_before_at() {
        let addr_spec = AddrSpec::from_str("jdoe @machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "white-spaces")]
    #[test]
    fn test_addr_spec_from_str_with_white_space_after_at() {
        let addr_spec = AddrSpec::from_str("jdoe@ machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "white-spaces")]
    #[test]
    fn test_addr_spec_from_str_with_white_space_after_domain() {
        let addr_spec = AddrSpec::from_str("jdoe@machine.example ").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_comments_before_local_part() {
        let addr_spec = AddrSpec::from_str("(John Doe)jdoe@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_comments_before_at() {
        let addr_spec = AddrSpec::from_str("jdoe(John Doe)@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_comments_after_at() {
        let addr_spec = AddrSpec::from_str("jdoe@(John Doe)machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_comments_after_domain() {
        let addr_spec = AddrSpec::from_str("jdoe@machine.example(John Doe)").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_nested_comments_before_local_part() {
        let addr_spec =
            AddrSpec::from_str("(John Doe (The Adventurer))jdoe@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_nested_comments_before_at() {
        let addr_spec =
            AddrSpec::from_str("jdoe(John Doe (The Adventurer))@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_nested_comments_after_at() {
        let addr_spec =
            AddrSpec::from_str("jdoe@(John Doe (The Adventurer))machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[cfg(feature = "comments")]
    #[test]
    fn test_addr_spec_from_str_with_nested_comments_after_domain() {
        let addr_spec =
            AddrSpec::from_str("jdoe@machine.example(John Doe (The Adventurer))").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[test]
    fn test_addr_spec_from_str_with_empty_labels() {
        let addr_spec = AddrSpec::from_str("\"..\"@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "..");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "\"..\"@machine.example");
    }

    #[test]
    fn test_addr_spec_from_str_with_quote() {
        let addr_spec = AddrSpec::from_str("\"jdoe\"@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@machine.example");
    }

    #[test]
    fn test_addr_spec_from_str_with_escape_and_quote() {
        let addr_spec = AddrSpec::from_str("\"jdoe\\\"\"@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe\"");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "\"jdoe\\\"\"@machine.example");
    }

    #[test]
    fn test_addr_spec_from_str_with_white_space_escape_and_quote() {
        let addr_spec = AddrSpec::from_str("\"jdoe\\ \"@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe ");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "\"jdoe\\ \"@machine.example");
    }

    #[cfg(not(feature = "white-spaces"))]
    #[test]
    fn test_addr_spec_from_str_with_white_spaces_and_white_space_escape_and_quote() {
        assert_eq!(
            AddrSpec::from_str("\"jdoe \\  \"@machine.example").unwrap_err(),
            ParseError("invalid character in quoted local part", 5)
        );
    }

    #[cfg(feature = "white-spaces")]
    #[test]
    fn test_addr_spec_from_str_with_white_spaces_and_white_space_escape_and_quote() {
        let addr_spec = AddrSpec::from_str("\"jdoe \\  \"@machine.example").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe ");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "\"jdoe\\ \"@machine.example");
    }

    #[cfg(feature = "literals")]
    #[test]
    fn test_addr_spec_from_str_with_domain_literal() {
        let addr_spec = AddrSpec::from_str("jdoe@[machine.example]").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@[machine.example]");
    }

    #[cfg(feature = "literals")]
    #[test]
    fn test_addr_spec_from_str_with_escape_and_domain_literal() {
        let addr_spec = AddrSpec::from_str("\"jdoe\"@[machine.example]").unwrap();
        assert_eq!(addr_spec.local_part(), "jdoe");
        assert_eq!(addr_spec.domain(), "machine.example");
        assert_eq!(addr_spec.to_string(), "jdoe@[machine.example]");
    }

    #[test]
    fn test_addr_spec_from_str_with_unicode() {
        let addr_spec = AddrSpec::from_str("😄😄😄@😄😄😄").unwrap();
        assert_eq!(addr_spec.local_part(), "😄😄😄");
        assert_eq!(addr_spec.domain(), "😄😄😄");
        assert_eq!(addr_spec.to_string(), "😄😄😄@😄😄😄");
    }

    #[test]
    fn test_addr_spec_from_str_with_escape_and_unicode() {
        let addr_spec = AddrSpec::from_str("\"😄😄😄\"@😄😄😄").unwrap();
        assert_eq!(addr_spec.local_part(), "😄😄😄");
        assert_eq!(addr_spec.domain(), "😄😄😄");
        assert_eq!(addr_spec.to_string(), "😄😄😄@😄😄😄");
    }

    #[test]
    fn test_addr_spec_from_str_with_escape_and_unicode_and_quote() {
        let addr_spec = AddrSpec::from_str("\"😄😄😄\\\"\"@😄😄😄").unwrap();
        assert_eq!(addr_spec.local_part(), "😄😄😄\"");
        assert_eq!(addr_spec.domain(), "😄😄😄");
        assert_eq!(addr_spec.to_string(), "\"😄😄😄\\\"\"@😄😄😄");
    }

    #[test]
    #[cfg(feature = "literals")]
    fn test_addr_spec_from_str_with_escape_and_unicode_and_domain_literal() {
        let addr_spec = AddrSpec::from_str("\"😄😄😄\"@[😄😄😄]").unwrap();
        assert_eq!(addr_spec.local_part(), "😄😄😄");
        assert_eq!(addr_spec.domain(), "😄😄😄");
        assert_eq!(addr_spec.to_string(), "😄😄😄@[😄😄😄]");
    }
}

#[cfg(all(test, feature = "nightly"))]
mod benches {
    extern crate test;

    use super::*;

    mod addr_spec {
        use super::*;

        #[bench]
        fn bench_trivial(b: &mut test::Bencher) {
            b.iter(|| {
                let address = AddrSpec::from_str("test@example.com").unwrap();
                assert_eq!(address.local_part(), "test");
                assert_eq!(address.domain(), "example.com");
                assert_eq!(address.to_string().as_str(), "test@example.com");
            });
        }

        #[bench]
        fn bench_quoted_local_part(b: &mut test::Bencher) {
            b.iter(|| {
                let address = AddrSpec::from_str("\"test\"@example.com").unwrap();
                assert_eq!(address.local_part(), "test");
                assert_eq!(address.domain(), "example.com");
                assert_eq!(address.to_string().as_str(), "test@example.com");
            });
        }

        #[cfg(feature = "literals")]
        #[bench]
        fn bench_literal_domain(b: &mut test::Bencher) {
            b.iter(|| {
                let address = AddrSpec::from_str("test@[example.com]").unwrap();
                assert_eq!(address.local_part(), "test");
                assert_eq!(address.domain(), "example.com");
                assert_eq!(address.to_string().as_str(), "test@[example.com]");
            });
        }

        #[cfg(feature = "literals")]
        #[bench]
        fn bench_full(b: &mut test::Bencher) {
            b.iter(|| {
                let address = AddrSpec::from_str("\"test\"@[example.com]").unwrap();
                assert_eq!(address.local_part(), "test");
                assert_eq!(address.domain(), "example.com");
                assert_eq!(address.to_string().as_str(), "test@[example.com]");
            });
        }
    }

    #[cfg(feature = "email_address")]
    mod email_address {
        use super::*;

        use ::email_address::EmailAddress;

        #[bench]
        fn bench_trivial(b: &mut test::Bencher) {
            b.iter(|| {
                let address = EmailAddress::from_str("test@example.com").unwrap();
                assert_eq!(address.local_part(), "test");
                assert_eq!(address.domain(), "example.com");
                assert_eq!(address.to_string().as_str(), "test@example.com");
            });
        }

        #[bench]
        fn bench_quoted_local_part(b: &mut test::Bencher) {
            b.iter(|| {
                let address = EmailAddress::from_str("\"test\"@example.com").unwrap();
                assert_eq!(address.local_part(), "\"test\"");
                assert_eq!(address.domain(), "example.com");
                assert_eq!(address.to_string().as_str(), "\"test\"@example.com");
            });
        }

        #[cfg(feature = "literals")]
        #[bench]
        fn bench_literal_domain(b: &mut test::Bencher) {
            b.iter(|| {
                let address = EmailAddress::from_str("test@[example.com]").unwrap();
                assert_eq!(address.local_part(), "test");
                assert_eq!(address.domain(), "[example.com]");
                assert_eq!(address.to_string().as_str(), "test@[example.com]");
            });
        }

        #[cfg(feature = "literals")]
        #[bench]
        fn bench_full(b: &mut test::Bencher) {
            b.iter(|| {
                let address = EmailAddress::from_str("\"test\"@[example.com]").unwrap();
                assert_eq!(address.local_part(), "\"test\"");
                assert_eq!(address.domain(), "[example.com]");
                assert_eq!(address.to_string().as_str(), "\"test\"@[example.com]");
            });
        }
    }

    // Sanity check that the regex is actually slower than the hand-written
    // parser. The regex below is a fairly simple one, but should still slower
    // than the hand-written parser.
    #[bench]
    fn bench_addr_spec_regexp(b: &mut test::Bencher) {
        use regex::Regex;

        let regex = Regex::new(r#"^(?:"(.*)"|([^@]+))@(?:\[(.*)\]|(.*))$"#).unwrap();
        b.iter(|| {
            {
                let captures = regex.captures("test@example.com").unwrap();
                assert_eq!(
                    unsafe {
                        AddrSpec::new_unchecked(
                            captures.get(2).unwrap().as_str(),
                            captures.get(4).unwrap().as_str(),
                        )
                    }
                    .to_string()
                    .as_str(),
                    "test@example.com"
                );
            }
            AddrSpec::from_str("test@example.com").unwrap();
            {
                let captures = regex.captures("\"test\"@example.com").unwrap();
                assert_eq!(
                    unsafe {
                        AddrSpec::new_unchecked(
                            captures.get(1).unwrap().as_str(),
                            captures.get(4).unwrap().as_str(),
                        )
                    }
                    .to_string()
                    .as_str(),
                    "test@example.com"
                );
            }
            #[cfg(feature = "literals")]
            {
                let captures = regex.captures("test@[example.com]").unwrap();
                assert_eq!(
                    unsafe {
                        AddrSpec::with_literal_unchecked(
                            captures.get(2).unwrap().as_str(),
                            captures.get(3).unwrap().as_str(),
                        )
                    }
                    .to_string()
                    .as_str(),
                    "test@[example.com]"
                );
            }
            #[cfg(feature = "literals")]
            {
                let captures = regex.captures("\"test\"@[example.com]").unwrap();
                assert_eq!(
                    unsafe {
                        AddrSpec::with_literal_unchecked(
                            captures.get(1).unwrap().as_str(),
                            captures.get(3).unwrap().as_str(),
                        )
                    }
                    .to_string()
                    .as_str(),
                    "test@[example.com]"
                );
            }
        });
    }
}
