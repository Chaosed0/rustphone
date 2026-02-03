#!/bin/bash

export RUST_BACKTRACE=1
./cargo.sh build && target/debug/game
