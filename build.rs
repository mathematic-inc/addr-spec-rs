use std::{env, path::PathBuf};

use rustc_version::{version_meta, Channel};

#[cfg(feature = "normalization")]
fn generate_icu_data() {
    use icu_datagen::SourceData;

    icu_datagen::datagen(
        None,
        &icu_datagen::keys(&[
            "normalizer/nfd@1",
            "normalizer/comp@1",
            "normalizer/nfdex@1",
        ]),
        &SourceData::default().with_icuexport_latest().unwrap(),
        vec![icu_datagen::Out::Module {
            mod_directory: PathBuf::from(env::var("OUT_DIR").unwrap()).join("addr_spec_icu"),
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
