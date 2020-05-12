#!/bin/bash

DIR1=~/sara-perf/openloop
DIR2=~/sara-perf/barrier
TRACKER_LOG_FILE=~/out
mkdir -p $DIR1 $DIR2

export TRACKER_LOG_FILE
export DIR1
export DIR2

# cargo run --release --example openloop-perf 10 10 > $DIR1/old_enabled
# cargo run --release --example barrier-perf 100 > $DIR2/old_enabled
hwloc-bind node:2.pu:even -- cargo run --release --example openloop-perf 100 100 > $DIR1/old_enabled
hwloc-bind node:2.pu:even -- cargo run --release --example barrier-perf 1000000 > $DIR2/old_enabled

# unset TRACKER_LOG_FILE
cargo run --release --example openloop-perf 10 10 > $DIR1/old_disabled
cargo run --release --example barrier-perf 100 > $DIR2/old_disabled
hwloc-bind node:2.pu:even -- cargo run --release --example openloop-perf 100 100 > $DIR1/old_disabled
hwloc-bind node:2.pu:even -- cargo run --release --example barrier-perf 1000000 > $DIR2/old_disabled