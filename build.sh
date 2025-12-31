#!/bin/bash

DIR="$(dirname "$0")"
export LIBCLANG_PATH="$PWD/lib"
export GNS_VCPKG_BUILDTREES_ROOT="/c/vcpkgroot"

if cargo build "$@"; then
    [ -d "$DIR/target/debug" ] && cp -r "$DIR/projects/client/assets" "$DIR/target/debug/assets"
    [ -d "$DIR/target/release" ] && cp -r "$DIR/projects/client/assets" "$DIR/target/release/assets"
fi
