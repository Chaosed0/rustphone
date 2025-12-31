#!/bin/bash

./build.sh && RUST_BACKTRACE=1 target/debug/game.exe
