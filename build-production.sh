#!/bin/sh

cargo build --release
cp -f ./target/release/rtmp-server rtmp-server
