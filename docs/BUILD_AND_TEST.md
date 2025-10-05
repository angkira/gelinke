# 🚀 Build & Test Guide

Quick reference for building and testing the firmware.

---

## 📦 **BUILD**

### **Production (Real Hardware):**
```bash
cargo build --release
cargo run --release  # Flash to hardware
```

### **Renode Testing (Mock Peripherals):**
```bash
cargo build --release --features renode-mock
```

**Key Differences:**
- Production: Real FDCAN @ iRPC, FOC @ 10 kHz
- Renode: Mock CAN + Mock FOC @ 1 Hz

---

## 🧪 **TEST**

### **Quick Test (Docker):**
```bash
docker compose run --rm renode bash -c "cargo build --release --features renode-mock && cd /workspace && renode-test renode/tests/basic_startup.robot"
```

### **Manual Test:**
```bash
./renode/manual_test.sh
```

### **Interactive Renode:**
```bash
docker compose run --rm renode bash
# Inside container:
cargo build --release --features renode-mock
renode renode/stm32g431_foc.resc
```

### **Native (if Renode installed):**
```bash
cargo build --release --features renode-mock
renode-test renode/tests/basic_startup.robot
```

---

## ✅ **EXPECTED RESULTS**

```
+++++ Finished test 'Should Boot And Show Banner' in X.XX seconds with status OK
+++++ Finished test 'Should Initialize System' in X.XX seconds with status OK
+++++ Finished test 'Should Start Heartbeat' in X.XX seconds with status OK
+++++ Finished test 'Should Initialize PWM' in X.XX seconds with status OK
+++++ Finished test 'Should Initialize CAN' in X.XX seconds with status OK

Tests finished successfully :)
```

---

## 🔍 **DEBUG**

### **View UART Output:**
```bash
docker compose run --rm renode bash -c "
  renode --disable-xwt --console -e '
    mach create
    machine LoadPlatformDescription @/workspace/renode/stm32g431cb.repl
    sysbus LoadELF @/workspace/target/thumbv7em-none-eabihf/release/joint_firmware
    showAnalyzer sysbus.usart1
    start
  '
"
```

### **GDB Debugging:**
```bash
# Terminal 1 - Start Renode with GDB server:
renode --console
(monitor) machine StartGdbServer 3333

# Terminal 2 - Connect GDB:
arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/joint_firmware
(gdb) target remote :3333
(gdb) break main
(gdb) continue
```

---

## 📊 **TEST COVERAGE**

Current: **5/5 tests passing**

| Test Suite | Status | Coverage |
|------------|--------|----------|
| `basic_startup.robot` | ✅ 5/5 | Boot, init, heartbeat, PWM, CAN |
| `can_communication.robot` | ⏳ TODO | CAN message TX/RX |
| `foc_control.robot` | ⏳ TODO | FOC loop, ADC, encoder |

---

## 🛠️ **TROUBLESHOOTING**

### **Build fails:**
```bash
# Clean and rebuild:
cargo clean
cargo build --release --features renode-mock
```

### **Tests hang:**
```bash
# Check if built with renode-mock feature:
grep "renode-mock" Cargo.toml
```

### **Docker issues:**
```bash
# Rebuild Docker image:
docker compose build --no-cache renode
```

### **UART no output:**
```bash
# Verify ELF has correct entry point:
docker compose run --rm renode bash -c "
  arm-none-eabi-readelf -h target/thumbv7em-none-eabihf/release/joint_firmware | grep Entry
"
# Should show: Entry point address: 0x8xxxxxx (NOT 0x0)
```

---

## 📚 **MORE INFO**

- Full setup: `LOCAL_TESTING.md`
- Docker setup: `DOCKER_SETUP.md`
- Platform details: `EXTENDED_PLATFORM_SUMMARY.md`
- Victory story: `FINAL_VICTORY.md`
