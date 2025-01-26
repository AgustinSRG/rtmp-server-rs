@echo off

call cargo build --release
cp -f .\target\release\rtmp-server.exe rtmp-server.exe
