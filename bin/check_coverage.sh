#!/bin/bash
# Coverage: reject local covers! macros, then run the suite and report (spec-trace run).
spec-trace check-macros --docs CLAUDE.md || exit 1
exec spec-trace run --threshold 100 "$@" -- cargo test --all-features --no-fail-fast
