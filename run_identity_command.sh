#!/bin/bash

set -a
source .env
set +a

cargo run --release --bin identity-server -- "$@"
