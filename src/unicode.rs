#[cfg(feature = "normalization")]
pub fn normalize<S>(value: S) -> String
where
    S: AsRef<str>,
{
    use unicode_normalization::UnicodeNormalization;
    value.as_ref().nfc().collect::<String>()
}

#[cfg(not(feature = "normalization"))]
pub fn normalize<S>(value: S) -> String
where
    S: Into<String>,
{
    value.into()
}
