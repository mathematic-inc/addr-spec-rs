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
    println!("cargo:rerun-if-changed=src/ascii.h");
    bindgen::Builder::default()
        .header("src/ascii.h")
        .blocklist_type("max_align_t")
        .blocklist_type("wchar_t")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("ascii.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed=src/ascii.c");
    cc::Build::new().file("src/ascii.c").compile("ascii");

    #[cfg(feature = "normalization")]
    generate_icu_data();

    if version_meta().unwrap().channel == Channel::Nightly {
        println!("cargo:rustc-cfg=feature=\"nightly\"");
    }
}
