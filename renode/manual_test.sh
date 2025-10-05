#!/bin/bash
################################################################################
# Manual testing script for Renode emulation
# Usage: ./renode/manual_test.sh [test_name]
################################################################################

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIRMWARE_DEBUG="$PROJECT_ROOT/target/thumbv7em-none-eabihf/debug/joint_firmware"
FIRMWARE_RELEASE="$PROJECT_ROOT/target/thumbv7em-none-eabihf/release/joint_firmware"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║ STM32G431CB FOC Controller - Renode Manual Testing                   ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if Renode is installed
if ! command -v renode &> /dev/null; then
    echo -e "${RED}ERROR: Renode is not installed${NC}"
    echo "Install from: https://github.com/renode/renode/releases"
    exit 1
fi

RENODE_VERSION=$(renode --version | head -n1)
echo -e "Renode version: ${GREEN}$RENODE_VERSION${NC}"
echo ""

# Build firmware if needed
if [ ! -f "$FIRMWARE_DEBUG" ]; then
    echo -e "${YELLOW}Building firmware (debug)...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --target thumbv7em-none-eabihf
    echo -e "${GREEN}✓ Build complete${NC}"
    echo ""
fi

# Show firmware info
if [ -f "$FIRMWARE_DEBUG" ]; then
    echo "Firmware binary: $FIRMWARE_DEBUG"
    if command -v arm-none-eabi-size &> /dev/null; then
        arm-none-eabi-size "$FIRMWARE_DEBUG"
    fi
    echo ""
fi

# Test selection
TEST_NAME="${1:-interactive}"

case "$TEST_NAME" in
    "interactive")
        echo -e "${GREEN}Starting interactive Renode session...${NC}"
        echo "Use Renode commands to inspect and control emulation"
        echo ""
        cd "$PROJECT_ROOT"
        renode renode/stm32g431_foc.resc
        ;;
    
    "basic")
        echo -e "${GREEN}Running basic startup tests...${NC}"
        cd "$PROJECT_ROOT"
        renode-test renode/tests/basic_startup.robot
        ;;
    
    "can")
        echo -e "${GREEN}Running CAN communication tests...${NC}"
        cd "$PROJECT_ROOT"
        renode-test renode/tests/can_communication.robot
        ;;
    
    "foc")
        echo -e "${GREEN}Running FOC control tests...${NC}"
        cd "$PROJECT_ROOT"
        renode-test renode/tests/foc_control.robot
        ;;
    
    "all")
        echo -e "${GREEN}Running all automated tests...${NC}"
        cd "$PROJECT_ROOT"
        renode-test renode/tests/*.robot
        ;;
    
    "build-test")
        echo -e "${GREEN}Build and quick test...${NC}"
        cd "$PROJECT_ROOT"
        cargo build --target thumbv7em-none-eabihf
        renode-test renode/tests/basic_startup.robot
        ;;
    
    *)
        echo -e "${RED}Unknown test: $TEST_NAME${NC}"
        echo ""
        echo "Available tests:"
        echo "  interactive  - Start Renode in interactive mode (default)"
        echo "  basic        - Run basic startup tests"
        echo "  can          - Run CAN communication tests"
        echo "  foc          - Run FOC control tests"
        echo "  all          - Run all automated tests"
        echo "  build-test   - Build firmware and run basic tests"
        echo ""
        echo "Usage: $0 [test_name]"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✓ Done${NC}"

