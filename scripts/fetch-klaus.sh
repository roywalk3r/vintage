#!/usr/bin/env bash
# Fetches the external correctness gate: Klaus Dormann's 6502 functional test.
# Third-party binary — not redistributed in this repo, downloaded per build.
set -euo pipefail

mkdir -p .cache
curl -sL -o .cache/6502_functional_test.bin \
  https://github.com/Klaus2m5/6502_65C02_functional_tests/raw/master/bin_files/6502_functional_test.bin
curl -sL -o .cache/klaus-readme.md \
  https://raw.githubusercontent.com/Klaus2m5/6502_65C02_functional_tests/master/readme.txt

echo "fetched: $(stat -c%s .cache/6502_functional_test.bin) bytes"