#!/usr/bin/bash
./target/debug/sunrise --demo &
PID=$!
sleep 3
scrot -u -o img/example.png
kill $PID