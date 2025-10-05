# Documentation Index

---

## 🚀 Quick Start

**New to Renode emulation?** Start here:
- **[README_RENODE.md](README_RENODE.md)** - Main entry point, overview & status
- **[QUICK_START.txt](QUICK_START.txt)** - One-page cheat sheet

---

## 📖 Guides

### **Setup & Installation:**
- **[LOCAL_TESTING.md](LOCAL_TESTING.md)** - Native Renode installation & testing
- **[DOCKER_SETUP.md](DOCKER_SETUP.md)** - Docker-based workflow (recommended)

### **Development:**
- **[BUILD_AND_TEST.md](BUILD_AND_TEST.md)** - Build modes & test execution
- **[EMULATION_QUICKSTART.md](EMULATION_QUICKSTART.md)** - Quick emulation guide

---

## 🎯 Use Cases

### **Emulation Options:**
- **[EMULATION_OPTIONS.md](EMULATION_OPTIONS.md)** - All emulation strategies (Renode, QEMU, Mock HAL)

---

## 📊 Project Status

- **[../STATUS.md](../STATUS.md)** - Overall project status & roadmap

---

## 🔧 Quick Commands

### **Docker (Recommended):**
```bash
# Run all tests
docker compose run --rm renode bash -c "
  cargo build --release --features renode-mock && 
  cd /workspace && 
  renode-test renode/tests/basic_startup.robot
"

# Interactive Renode
docker compose run --rm renode bash
```

### **Native:**
```bash
# Build for Renode
cargo build --release --features renode-mock

# Run tests
renode-test renode/tests/basic_startup.robot
```

---

## ✅ Current Status

- **Tests:** 5/5 passing ✅
- **Platform:** STM32G431CB with 42+ peripherals
- **Embassy:** Fully functional async executor
- **Build modes:** Production & Renode-mock

---

**Need help?** See [README_RENODE.md](README_RENODE.md) for detailed documentation.
