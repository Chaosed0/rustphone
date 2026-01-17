#!/bin/bash

./build.sh && RUST_BACKTRACE=1 cargo run --bin game
