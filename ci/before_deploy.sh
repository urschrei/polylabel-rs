# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    if [[ "$TRAVIS_OS_NAME" == "linux" ]]; then
        # we've already built release binaries in Docker, so no-op
        echo "Linux release artifacts built in manylinux1 Docker image"
    fi
    if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
        # TODO Update this to build the artifacts that matter to you
        cross rustc --target $TARGET --release
        for lib in target/$TARGET/release/*.dylib; do
            strip -ur $lib
        done
        cp target/$TARGET/release/*.dylib $stage
        # TODO Update this to package the right artifacts
        cp -r target/$TARGET/release/*.dSYM $stage 2>/dev/null || :
        cd $stage
        tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
        cd $src
    fi

    rm -rf $stage
}

main
