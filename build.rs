extern crate cbindgen;

use std::env;

fn write_headers() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let config = cbindgen::Config::from_file("cbindgen.toml").unwrap();

    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Unable to generate bindings")
        .write_to_file("include/header.h");
}

fn main() {
    let headers_enabled = env::var_os("CARGO_FEATURE_HEADERS").is_some();
    if headers_enabled {
        write_headers();
    }
}
