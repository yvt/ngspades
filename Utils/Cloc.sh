#!/bin/sh

set -x
cd "$(dirname "$0")/.."

which loc > /dev/null || {
    echo "Fatal: Could not locate the 'loc' tool. You can install it by 'cargo install loc'."
    exit 1
}

loc
