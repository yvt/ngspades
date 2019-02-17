#!/bin/sh

set -x
cd "$(dirname "$0")/.."

which tokei > /dev/null || {
    echo "Fatal: Could not locate the 'tokei' tool. You can install it by 'cargo install tokei'."
    exit 1
}

tokei
