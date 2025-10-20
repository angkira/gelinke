#!/usr/bin/env bash
# Run Renode E2E tests with FOC visualization
set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_RESULTS_DIR="${PROJECT_DIR}/test_results"
RENODE_TESTS_DIR="${PROJECT_DIR}/renode/tests"

echo "========================================"
echo "FOC E2E Tests with Visualization"
echo "========================================"
echo ""

# Create results directory
mkdir -p "${TEST_RESULTS_DIR}"

# Check dependencies
echo "Checking dependencies..."

if ! command -v python3 &> /dev/null; then
    echo "❌ Python3 not found"
    exit 1
fi

# Check Python packages
python3 -c "import matplotlib" 2>/dev/null || {
    echo "❌ matplotlib not found. Installing..."
    pip3 install matplotlib
}

python3 -c "import pandas" 2>/dev/null || {
    echo "❌ pandas not found. Installing..."
    pip3 install pandas
}

python3 -c "import numpy" 2>/dev/null || {
    echo "❌ numpy not found. Installing..."
    pip3 install numpy
}

python3 -c "import scipy" 2>/dev/null || {
    echo "❌ scipy not found. Installing..."
    pip3 install scipy
}

echo "✓ All dependencies installed"
echo ""

# Build firmware
echo "Building firmware..."
cd "${PROJECT_DIR}"

if [ ! -f "target/thumbv7em-none-eabihf/release/joint_firmware" ]; then
    echo "Building release firmware..."
    cargo build --release
else
    echo "✓ Firmware already built"
fi

echo ""

# Run tests with Robot Framework
echo "Running E2E tests with data collection..."
echo ""

cd "${RENODE_TESTS_DIR}"

# Run tests with visualization keywords
robot \
    --outputdir "${TEST_RESULTS_DIR}" \
    --variable TEST_RESULTS_DIR:"${TEST_RESULTS_DIR}" \
    --loglevel DEBUG \
    --consolecolors on \
    example_motion_test_with_viz.robot

TEST_EXIT_CODE=$?

echo ""
echo "========================================"
echo "Test Execution Complete"
echo "========================================"
echo ""

# Generate reports for all test data
echo "Generating FOC visualization reports..."
echo ""

cd "${PROJECT_DIR}"

# Find all test JSON files
for json_file in "${TEST_RESULTS_DIR}"/*.json; do
    if [ -f "$json_file" ] && [[ ! "$json_file" =~ "summary" ]]; then
        test_name=$(basename "$json_file" .json)
        pdf_file="${TEST_RESULTS_DIR}/${test_name}_report.pdf"
        
        echo "📊 Generating report: ${test_name}_report.pdf"
        
        python3 "${RENODE_TESTS_DIR}/test_report_generator.py" \
            --input "$json_file" \
            --output "$pdf_file"
        
        if [ $? -eq 0 ]; then
            echo "   ✓ Report generated"
        else
            echo "   ❌ Failed to generate report"
        fi
    fi
done

echo ""

# Generate suite summary
echo "📊 Generating test suite summary..."
python3 "${RENODE_TESTS_DIR}/test_report_generator.py" \
    --input "${TEST_RESULTS_DIR}" \
    --suite-summary \
    --output "${TEST_RESULTS_DIR}/test_suite_summary.pdf"

if [ $? -eq 0 ]; then
    echo "   ✓ Suite summary generated"
else
    echo "   ❌ Failed to generate suite summary"
fi

echo ""
echo "========================================"
echo "Results Summary"
echo "========================================"
echo ""
echo "Test results: ${TEST_RESULTS_DIR}"
echo ""

# List generated reports
if ls "${TEST_RESULTS_DIR}"/*.pdf 1> /dev/null 2>&1; then
    echo "Generated reports:"
    for pdf in "${TEST_RESULTS_DIR}"/*.pdf; do
        echo "  📄 $(basename "$pdf")"
    done
else
    echo "⚠️  No PDF reports generated"
fi

echo ""

# List data files
if ls "${TEST_RESULTS_DIR}"/*.json 1> /dev/null 2>&1; then
    echo "Collected data:"
    for json in "${TEST_RESULTS_DIR}"/*.json; do
        echo "  📊 $(basename "$json")"
    done
else
    echo "⚠️  No test data collected"
fi

echo ""
echo "========================================"

if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo "✅ All tests passed with visualization"
else
    echo "❌ Some tests failed (exit code: $TEST_EXIT_CODE)"
fi

echo "========================================"
echo ""

# Optionally open summary report (on Linux with PDF viewer)
if command -v xdg-open &> /dev/null; then
    read -p "Open test suite summary report? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        xdg-open "${TEST_RESULTS_DIR}/test_suite_summary.pdf" &
    fi
fi

exit $TEST_EXIT_CODE

