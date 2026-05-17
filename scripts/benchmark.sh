#!/usr/bin/env bash

hyperfine --warmup 3 \
  'seq 1 100 | rust-parallel echo' \
  'seq 1 100 | xargs -P8 -L1 echo' \
  'seq 1 100 | parallel echo'
