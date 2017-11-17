#!/bin/sh
DISPLAY=:1.0 RUST_BACKTRACE=1 RUST_LOG=debug exec ./target/debug/gwm-kbd 2> wm_log > tags_log
