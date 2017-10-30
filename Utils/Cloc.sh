#!/bin/sh

set -x
cd "$(dirname "$0")/.."

# TODO: do not exclude `bin` directories in NGSEngine/src
cloc --exclude-dir=.vscode,bin,target,Derived .
