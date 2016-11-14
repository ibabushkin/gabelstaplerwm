#!/bin/sh
DISPLAY=:1.0 RUST_BACKTRACE=1 RUST_LOG=debug exec ./target/debug/gabelstaplerwm 2> wm_log
