#!/bin/bash
# Coverage: reject local covers! macros, then run the suite and report (spec-trace run).
# The test command is fixed (--all-features keeps coverage at 100%); arguments go to spec-trace run as options only, and `-- <cmd>` cannot replace it.
spec-trace check-macros --docs CLAUDE.md || exit 1
exec spec-trace run --threshold 100 "$@" -- cargo test --all-features --no-fail-fast
