#!/usr/bin/env bash
set -e -u

SUDO=
[ "$(id -u)" = 0 ] || SUDO=sudo

NEED=( cargo libssl-dev pkg-config )
WANT=( rust-gdb entr )

set -x
$SUDO apt update
$SUDO apt install "${NEED[@]}" "${WANT[@]}"
