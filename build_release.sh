#!/bin/bash

TARGETS="x86_64-unknown-linux-gnu x86_64-pc-windows-gnu"
for TARGET in $TARGETS; do
  cargo build --target $TARGET --release
done
