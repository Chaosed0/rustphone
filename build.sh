#!/bin/bash

DIR="$(dirname "$0")"
set -o allexport && source .env && set +o allexport

cargo build "$@"
#cargo flamegraph -p game

BUILD_RESULT=$?

if [ $BUILD_RESULT -eq 0 ] ; then
    [ -d "$DIR/target/debug" ] && mkdir -p "$DIR/target/debug/assets/" && cp -r $DIR/projects/client/assets/* "$DIR/target/debug/assets/"
    [ -d "$DIR/target/release" ] && mkdir -p "$DIR/target/release/assets/" && cp -r $DIR/projects/client/assets/* "$DIR/target/release/assets/"
fi

exit $BUILD_RESULT
