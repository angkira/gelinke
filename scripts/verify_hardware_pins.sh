#!/bin/bash
# CLN17 V2.0 Hardware Pin Verification Script
# Validates firmware pin mappings match official hardware specification

set -e

echo "=========================================="
echo "CLN17 V2.0 Hardware Pin Verification"
echo "=========================================="
echo ""

ERRORS=0

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_pins() {
    local file=$1
    local pin=$2
    local description=$3

    if grep -q "$pin" "$file"; then
        echo -e "${GREEN}✅${NC} $description: $pin"
        return 0
    else
        echo -e "${RED}❌${NC} $description: $pin NOT FOUND in $file"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

# Check PWM pins (TIM2 for DRV8844 H-bridge)
echo "### PWM Motor Control (DRV8844 H-bridge)"
check_pins "src/firmware/drivers/pwm.rs" "PA0" "AIN1 (Phase A forward)"
check_pins "src/firmware/drivers/pwm.rs" "PA1" "AIN2 (Phase A reverse)"
check_pins "src/firmware/drivers/pwm.rs" "PB10" "BIN2 (Phase B reverse)"
check_pins "src/firmware/drivers/pwm.rs" "PB11" "BIN1 (Phase B forward)"

if grep -q "TIM2" src/firmware/drivers/pwm.rs; then
    echo -e "${GREEN}✅${NC} Using TIM2 (correct for CLN17 V2.0)"
else
    echo -e "${RED}❌${NC} NOT using TIM2 timer!"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Check ADC pins
echo "### ADC Current Sensing & Vbus"
check_pins "src/firmware/drivers/adc.rs" "PA3" "Phase A current sense (AISEN)"
check_pins "src/firmware/drivers/adc.rs" "PB0" "Phase B current sense (BISEN)"
check_pins "src/firmware/drivers/adc.rs" "PA2" "Vbus voltage monitoring"
echo ""

# Check motor driver control pins
echo "### Motor Driver Control (DRV8844)"
check_pins "src/firmware/drivers/motor_driver.rs" "PA4" "nSLEEP (enable)"
check_pins "src/firmware/drivers/motor_driver.rs" "PB1" "nFAULT (fault detection)"
check_pins "src/firmware/drivers/motor_driver.rs" "PB2" "nRESET (reset control)"
echo ""

# Check Step-Dir interface pins
echo "### Step-Dir Interface"
check_pins "src/firmware/drivers/step_dir_interface.rs" "PB5" "STEP pulse input"
check_pins "src/firmware/drivers/step_dir_interface.rs" "PB4" "DIR direction input"
check_pins "src/firmware/drivers/step_dir_interface.rs" "PA8" "ENABLE input"
check_pins "src/firmware/drivers/step_dir_interface.rs" "PB3" "ERROR output"
echo ""

# Check encoder SPI pins
echo "### Encoder SPI (TLE5012B)"
check_pins "src/firmware/drivers/sensors.rs" "PC4" "SPI CS (chip select)"
if grep -q "PA4" src/firmware/drivers/sensors.rs && ! grep -q "PC4" src/firmware/drivers/sensors.rs; then
    echo -e "${RED}❌${NC} CRITICAL: Encoder CS still on PA4 (conflicts with motor enable!)"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Check CAN pins
echo "### CAN Communication (FDCAN1)"
check_pins "src/firmware/system.rs" "PB8" "FDCAN1_RX"
check_pins "src/firmware/system.rs" "PB9" "FDCAN1_TX"

if grep -q "PA11.*CAN\|PA12.*CAN" src/firmware/system.rs; then
    echo -e "${RED}❌${NC} CRITICAL: CAN still on PA11/PA12 (conflicts with USB!)"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Check UART pins
echo "### UART Debug"
if grep -q "USART3" src/firmware/system.rs; then
    echo -e "${GREEN}✅${NC} Using USART3 (correct)"
else
    echo -e "${RED}❌${NC} Not using USART3!"
    ERRORS=$((ERRORS + 1))
fi

check_pins "src/firmware/system.rs" "PC10" "USART3 TX"
check_pins "src/firmware/system.rs" "PC11" "USART3 RX"

if grep -q "PA9.*UART\|PA10.*UART" src/firmware/system.rs; then
    echo -e "${YELLOW}⚠️${NC} Warning: UART may still reference PA9/PA10"
fi
echo ""

# Check Status LEDs
echo "### Status LEDs (RGB)"
check_pins "src/firmware/drivers/status_leds.rs" "PB13" "Red LED"
check_pins "src/firmware/drivers/status_leds.rs" "PB14" "Green LED"
check_pins "src/firmware/drivers/status_leds.rs" "PB15" "Blue LED"
echo ""

# Check CAN transceiver control (optional but should exist)
echo "### CAN Transceiver Control"
if [ -f "src/firmware/drivers/can_transceiver.rs" ]; then
    echo -e "${GREEN}✅${NC} CAN transceiver module exists"
    check_pins "src/firmware/drivers/can_transceiver.rs" "PA9" "CAN_SHDN (shutdown control)"
    check_pins "src/firmware/drivers/can_transceiver.rs" "PA10" "CAN_S (mode select)"
else
    echo -e "${YELLOW}⚠️${NC} CAN transceiver control not implemented (optional)"
fi
echo ""

# Check USB pins are NOT used elsewhere
echo "### USB Pin Conflict Check"
if grep -q "PA11" src/firmware/drivers/*.rs src/firmware/system.rs 2>/dev/null | grep -v "USB"; then
    echo -e "${RED}❌${NC} PA11 used for non-USB purpose (conflicts with USB D-)"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✅${NC} PA11 reserved for USB"
fi

if grep -q "PA12" src/firmware/drivers/*.rs src/firmware/system.rs 2>/dev/null | grep -v "USB"; then
    echo -e "${RED}❌${NC} PA12 used for non-USB purpose (conflicts with USB D+)"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✅${NC} PA12 reserved for USB"
fi
echo ""

# Summary
echo "=========================================="
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ All hardware pins verified successfully!${NC}"
    echo "Firmware matches CLN17 V2.0 hardware specification."
    exit 0
else
    echo -e "${RED}❌ Found $ERRORS hardware pin errors!${NC}"
    echo "Please review firmware pin mappings."
    exit 1
fi
