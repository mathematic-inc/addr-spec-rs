#[cfg(feature = "normalization")]
pub fn normalize<S>(value: S) -> String
where
    S: AsRef<str>,
{
    use icu_normalizer::ComposingNormalizer;
    const NORMALIZER: ComposingNormalizer = ComposingNormalizer::new_nfc();
    NORMALIZER.normalize(value.as_ref())
}

#[cfg(not(feature = "normalization"))]
pub fn normalize<S>(value: S) -> String
where
    S: Into<String>,
{
    value.into()
}
