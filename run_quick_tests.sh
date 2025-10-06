#!/bin/bash
# Quick test runner - unit tests only, no Renode required

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║       🧪 Joint Firmware Quick Test Suite 🧪                  ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."
if ! command -v cargo &> /dev/null; then
    echo "❌ cargo not found. Install Rust toolchain."
    exit 1
fi

if ! command -v robot &> /dev/null; then
    echo "❌ robot not found. Install: pip install robotframework"
    exit 1
fi

echo "✅ Prerequisites OK"
echo ""

# Run tests
echo "🧪 Running unit tests..."
echo ""

robot \
    --outputdir target/test-results \
    --loglevel INFO \
    --consolecolors on \
    renode/tests/simple_unit_tests.robot

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║                   ✅ TEST SUITE COMPLETE                      ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 View detailed results:"
echo "   - Report: target/test-results/report.html"
echo "   - Log:    target/test-results/log.html"
echo ""

