#![allow(clippy::unnecessary_lazy_evaluations)]

#[cfg(feature = "normalization")]
mod icu {
    use icu_normalizer::ComposingNormalizer;

    include!(concat!(env!("OUT_DIR"), "/addr_spec_icu/mod.rs"));

    thread_local! {
        pub static NORMALIZER: ComposingNormalizer = unsafe {
            ComposingNormalizer::try_new_nfc_unstable(&BakedDataProvider).unwrap_unchecked()
        };
    }
}

#[cfg(feature = "normalization")]
#[inline]
pub fn normalize<S>(value: S) -> String
where
    S: AsRef<str>,
{
    icu::NORMALIZER.with(|normalizer| normalizer.normalize(value.as_ref()))
}

#[cfg(not(feature = "normalization"))]
#[inline]
pub fn normalize<S>(value: S) -> String
where
    S: Into<String>,
{
    value.into()
}
