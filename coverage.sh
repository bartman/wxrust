#!/usr/bin/env bash
set -e -x
cargo tarpaulin --exclude-files "src/main.rs" --exclude-files "src/api.rs" --out Html --output-dir coverage/
