#!/bin/bash
# Check test coverage using spec-trace CLI with SQLite database
# Based on spec-trace-guide.md

set -e

echo "📊 MCP Planner Test Coverage Report"
echo "===================================="
echo ""

# Clean old database
echo "🧹 Cleaning old coverage database..."
rm -f coverage.db*

# Run tests with coverage tracking
echo "🧪 Running tests with coverage tracking..."
SPEC_TRACE_DB=coverage.db cargo test --quiet

if [ ! -f "coverage.db" ]; then
  echo "❌ Error: coverage.db was not created."
  echo "   Likely causes:"
  echo "   - tests are not calling spec_trace::covers (or an equivalent wrapper)"
  echo "   - spec-trace runtime coverage is disabled"
  echo "   - SPEC_TRACE_DB is not being honored in the test process"
  exit 1
fi

echo ""
echo "📈 Analyzing coverage..."
echo ""

# Read spec file list (shared source of truth with build.rs)
SPEC_FILES="spec-files.txt"
if [ ! -f "$SPEC_FILES" ]; then
  echo "❌ Error: $SPEC_FILES not found."
  exit 1
fi

# Build --specs arguments from the file (skip comments and blank lines)
SPECS_ARGS=()
while IFS= read -r line; do
  line="${line## }"  # trim leading spaces
  [[ -z "$line" || "$line" == \#* ]] && continue
  SPECS_ARGS+=(--specs "$line")
done < "$SPEC_FILES"

# Run coverage analysis
if [ ! -f "../spec-trace/Cargo.toml" ]; then
  echo "❌ Error: ../spec-trace/Cargo.toml not found."
  echo "   This script expects the spec-trace repository to be checked out at ../spec-trace."
  exit 1
fi

cargo run --manifest-path ../spec-trace/Cargo.toml --bin spec-trace -- coverage \
  --db coverage.db \
  "${SPECS_ARGS[@]}"
