use rustc_version::{version_meta, Channel};

#[cfg(feature = "normalization")]
fn generate_icu_data() {
    use icu_datagen::SourceData;

    use std::{env, path::PathBuf};

    let mod_directory = PathBuf::from(env::var("OUT_DIR").unwrap()).join("addr_spec_icu");
    if mod_directory.exists() {
        return;
    }

    icu_datagen::datagen(
        None,
        &icu_datagen::keys(&[
            "normalizer/nfd@1",
            "normalizer/comp@1",
            "normalizer/nfdex@1",
        ]),
        &SourceData::default()
            .with_icuexport_for_tag(SourceData::LATEST_TESTED_ICUEXPORT_TAG)
            .unwrap(),
        vec![icu_datagen::Out::Module {
            mod_directory,
            pretty: false,
            insert_feature_gates: false,
            use_separate_crates: true,
        }],
    )
    .unwrap();
}

fn main() {
    #[cfg(feature = "normalization")]
    generate_icu_data();

    if version_meta().unwrap().channel == Channel::Nightly {
        println!("cargo:rustc-cfg=feature=\"nightly\"");
    }
}
