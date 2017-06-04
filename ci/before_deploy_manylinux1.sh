#!/bin/bash
set -ex

export CRATE_NAME=polylabel-rs
# we pass {TRAVIS_TAG} into Docker from Travis
export TARGET=x86_64-unknown-linux-gnu

export PATH="$PATH:$HOME/.cargo/bin"
# we always produce release artifacts using stable
export TRAVIS_RUST_VERSION=stable

# coreutils configure whines otherwise 
export FORCE_UNSAFE_CONFIGURE=1

install_rustup() {
    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source ~/.cargo/env || true
    rustc -V
}

# Generate artefacts for release
mk_artifacts() {
    ls $HOME/.cargo/bin
    RUSTFLAGS='-C target-cpu=native' cargo build --manifest-path=/io/Cargo.toml --target $TARGET --release
}

mk_tarball() {
    # create a "staging" directory
    local td=$(echo $(mktemp -d 2>/dev/null || mktemp -d -t tmp))
    local out_dir=/io$(pwd)

    # TODO update this part to copy the artifacts that make sense for your project
    # NOTE All Cargo build artifacts will be under the 'target/$TARGET/{debug,release}'
    for lib in /io/target/$TARGET/release/*.so; do
        strip -s $lib
    done

    cp /io/target/$TARGET/release/*.so $td
    cp -r /io/target/$TARGET/release/*.dSYM $td 2>/dev/null || :
    cp /io/include/header.h $td

    pushd $td
    # release tarball will look like 'rust-everywhere-v1.2.3-x86_64-unknown-linux-gnu.tar.gz'
    tar czf /io/${CRATE_NAME}-${TRAVIS_TAG}-${TARGET}.tar.gz *

    popd
    rm -r $td
}

main() {
    install_rustup
    mk_artifacts
    mk_tarball
}
main
