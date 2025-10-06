#!/bin/bash
# Quick test runner for joint_firmware

set -e

echo "🔧 Building firmware with renode-mock..."
cargo build --release --features renode-mock

echo ""
echo "🧪 Running Renode tests..."

# Check if renode-test is available
if ! command -v renode-test &> /dev/null; then
    echo "❌ renode-test not found. Install Renode:"
    echo "   https://renode.io/"
    exit 1
fi

# Run tests
TEST_RESULTS_DIR="target/test-results"
mkdir -p "$TEST_RESULTS_DIR"

echo ""
echo "📋 Running test suites..."
echo ""

# Basic startup tests (should pass)
echo "1️⃣ Basic Startup Tests..."
renode-test renode/tests/basic_startup.robot \
    --variable FIRMWARE_ELF:target/thumbv7em-none-eabihf/release-mock/joint_firmware \
    --outputdir "$TEST_RESULTS_DIR" \
    --loglevel DEBUG || echo "⚠️ Some tests failed"

echo ""
echo "2️⃣ Motion Planning Tests..."
renode-test renode/tests/motion_planning.robot \
    --variable FIRMWARE_ELF:target/thumbv7em-none-eabihf/release-mock/joint_firmware \
    --outputdir "$TEST_RESULTS_DIR" \
    --loglevel DEBUG || echo "⚠️ Some tests failed"

echo ""
echo "3️⃣ Telemetry Streaming Tests..."
renode-test renode/tests/telemetry_streaming.robot \
    --variable FIRMWARE_ELF:target/thumbv7em-none-eabihf/release-mock/joint_firmware \
    --outputdir "$TEST_RESULTS_DIR" \
    --loglevel DEBUG || echo "⚠️ Some tests failed"

echo ""
echo "4️⃣ Adaptive Control Tests..."
renode-test renode/tests/adaptive_control.robot \
    --variable FIRMWARE_ELF:target/thumbv7em-none-eabihf/release-mock/joint_firmware \
    --outputdir "$TEST_RESULTS_DIR" \
    --loglevel DEBUG || echo "⚠️ Some tests failed"

echo ""
echo "✅ Test run complete! Results in: $TEST_RESULTS_DIR"
echo ""
echo "📊 View detailed results:"
echo "   - Report: $TEST_RESULTS_DIR/report.html"
echo "   - Log:    $TEST_RESULTS_DIR/log.html"

