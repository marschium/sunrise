#!/usr/bin/bash
./target/debug/AirhornNotes --demo &
PID=$!
sleep 3
scrot -u -o img/example.png
kill $PID