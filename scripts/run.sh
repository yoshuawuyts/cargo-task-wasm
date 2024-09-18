#!/bin/bash

set -e

export MY_ENV_VAR="my env is this"

PATH=$PATH:$(pwd)/target/debug
cargo build
cd example
cargo task $@
