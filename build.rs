#[cfg(feature = "cbindgen")]
fn main() {
    use std::env;

    let config = cbindgen::Config::from_file("cbindgen.toml").unwrap();

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("include/header.h");
}

#[cfg(not(feature = "cbindgen"))]
fn main() {}
