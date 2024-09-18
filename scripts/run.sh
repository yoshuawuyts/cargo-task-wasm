#!/bin/bash

set -e

PATH=$PATH:$(pwd)/target/debug
cargo build
cd example
cargo task $@
