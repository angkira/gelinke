# 🎯 Промпт для создания комплексных тестов прошивки

Скопируй этот промпт в новый чат:

---

# Контекст проекта: STM32G431CB FOC Motor Controller + Renode Testing

## 📋 Что уже есть

### ✅ Работающая прошивка:
- **MCU:** STM32G431CB @ 170 MHz
- **Framework:** Embassy (async Rust)
- **Protocol:** iRPC over CAN-FD
- **Control:** FOC (Field-Oriented Control) @ 10 kHz
- **Sensors:** ADC (current), Encoder (position)
- **Actuators:** 3-phase complementary PWM
- **Logging:** UART @ 115200 baud

### ✅ Renode платформа:
- **42+ peripherals** эмулируются (86% ready-made)
- **UART, FDCAN, GPIO, Timers, ADC, SPI, I2C** и т.д.
- **Mock режим:** CAN @ no-blocking, FOC @ 1 Hz (для тестов)
- **Production режим:** Real FDCAN + FOC @ 10 kHz

### ✅ Текущие тесты (5/5 passing):
```robot
*** Test Cases ***
Should Boot And Show Banner          # Проверка загрузки и UART
Should Initialize System             # Проверка инициализации
Should Start Heartbeat               # Проверка системного таймера
Should Initialize PWM                # Проверка PWM инициализации
Should Initialize CAN                # Проверка CAN инициализации
```

**Файлы:** `renode/tests/basic_startup.robot`

---

## 🎯 ЗАДАЧА

Создай **КОМПЛЕКСНЫЕ ТЕСТЫ** для прошивки, покрывающие:

### 1. **CAN Communication (iRPC Protocol):**
   - Отправка/прием CAN-FD сообщений
   - iRPC request/response цикл
   - Обработка команд (SetVelocity, SetTorque, GetState)
   - Broadcast сообщений (heartbeat, telemetry)
   - Timeout handling
   - Error handling (CAN bus off, etc.)

### 2. **FOC Control Loop:**
   - Запуск FOC task @ 10 kHz (в mock режиме @ 1 Hz)
   - Калибровка ADC (zero current offsets)
   - Чтение токов фаз (ADC1, ADC2)
   - Чтение энкодера (position, velocity)
   - Clarke/Park преобразования
   - PI контроллеры (D/Q токи)
   - Inverse Park + SVPWM
   - PWM output (3 фазы + dead time)
   - State machine (Idle → Calibrating → Running → Fault)

### 3. **Sensor Integration:**
   - ADC continuous mode @ 10 kHz
   - Encoder reading (SPI или ABI)
   - Angle estimation
   - Velocity calculation (дифференцирование)
   - Sensor fault detection

### 4. **Safety & Fault Handling:**
   - Overcurrent detection (ADC thresholds)
   - Overvoltage/Undervoltage
   - Communication timeout (CAN watchdog)
   - Emergency stop (immediate PWM disable)
   - Fault recovery
   - Fault state persistence

### 5. **Performance & Timing:**
   - FOC loop latency (10 kHz = 100 µs period)
   - CAN message latency
   - Interrupt priorities
   - DMA performance
   - System load (CPU usage)

---

## 📂 Структура проекта

```
joint_firmware/
├── src/firmware/
│   ├── system.rs                 # System init, task spawning
│   ├── tasks/
│   │   ├── can_comm.rs           # CAN task (iRPC transport)
│   │   ├── foc.rs                # FOC control loop @ 10 kHz
│   │   ├── mock_can.rs           # Mock CAN for Renode
│   │   └── mock_foc.rs           # Mock FOC @ 1 Hz for Renode
│   ├── drivers/
│   │   ├── pwm.rs                # 3-phase complementary PWM (TIM1)
│   │   ├── adc.rs                # Current sensors (ADC1/ADC2)
│   │   ├── sensors.rs            # Encoder interface
│   │   └── can.rs                # (deprecated, replaced by iRPC)
│   ├── control/
│   │   ├── velocity.rs           # Velocity controller
│   │   └── position.rs           # Position controller
│   ├── hardware/
│   │   ├── cordic.rs             # CORDIC for trig (Clarke/Park)
│   │   └── fmac.rs               # FMAC for PI controllers
│   └── uart_log.rs               # UART logging for Renode
├── renode/
│   ├── stm32g431cb.repl          # Platform description (42+ peripherals)
│   ├── stm32g431_foc.resc        # Startup script
│   └── tests/
│       ├── basic_startup.robot   # ✅ 5/5 passing
│       ├── can_communication.robot   # TODO
│       └── foc_control.robot         # TODO
└── docs/
    ├── README_RENODE.md          # Основная документация
    ├── BUILD_AND_TEST.md         # Build & test guide
    └── EMULATION_OPTIONS.md      # Опции эмуляции
```

---

## 🛠️ Технические детали

### **Build modes:**
```bash
# Production (real hardware)
cargo build --release

# Renode testing (mock peripherals)
cargo build --release --features renode-mock
```

### **Mock vs Real:**
| Component | Production | Renode Mock |
|-----------|-----------|-------------|
| CAN | iRPC CanFdTransport | Mock (no async-wait) |
| FOC | 10 kHz loop | 1 Hz loop |
| ADC | Real DMA | Emulated |
| Encoder | Real SPI/ABI | Emulated |
| PWM | Real TIM1 | Emulated |

### **Renode peripherals:**
- `sysbus.usart1` - UART logging (115200 baud)
- `sysbus.fdcan1` - CAN.MCAN (CAN-FD controller)
- `sysbus.canMessageRAM` - FDCAN Message RAM
- `sysbus.adc1`, `sysbus.adc2` - Analog.STM32_ADC
- `sysbus.tim1` - Timers.STM32_Timer (PWM)
- `sysbus.gpioPortA-F` - GPIOPort.STM32_GPIOPort

### **iRPC Protocol:**
```rust
// Command examples:
SetVelocity { target: 100.0 }   // rad/s
SetTorque { target: 1.5 }       // Nm
GetState { }                    // → { position, velocity, current }
EmergencyStop { }
```

**Node ID:** `0x01` (DEFAULT_NODE_ID)

---

## 📝 Требования к тестам

### **1. Robot Framework (.robot файлы):**
```robot
*** Settings ***
Documentation     CAN Communication Tests
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}

*** Test Cases ***
Should Send CAN Frame
    [Documentation]    Test CAN-FD frame transmission
    # ... test implementation
```

### **2. Покрытие:**
- ✅ Положительные сценарии (happy path)
- ✅ Негативные сценарии (errors, timeouts)
- ✅ Граничные условия (max current, max velocity)
- ✅ Edge cases (zero velocity, direction change)

### **3. Assertions:**
- UART log messages (via `Wait For Line On Uart`)
- Register values (via `Execute Command`)
- Timing constraints (timeouts)
- State transitions

### **4. Helpers:**
Создай вспомогательные Python peripherals для:
- **Mock Encoder:** Генерация synthetic position data
- **Mock Current Sensors:** Синтетические токи фаз
- **CAN Message Injector:** Отправка iRPC команд в FDCAN
- **State Verifier:** Проверка внутреннего состояния

---

## 📚 Референсы

### **Документация:**
- `docs/README_RENODE.md` - Полная документация Renode setup
- `docs/BUILD_AND_TEST.md` - Как собирать и запускать тесты
- `STATUS.md` - Статус проекта и архитектура

### **Примеры тестов:**
- `renode/tests/basic_startup.robot` - 5 passing tests
- `renode/CHEATSHEET.md` - Renode commands

### **Существующий код:**
- `src/firmware/tasks/foc.rs` - FOC controller (см. FocController struct)
- `src/firmware/tasks/can_comm.rs` - CAN task с iRPC
- `src/firmware/uart_log.rs` - UART logging

---

## 🎯 Конкретные задачи

### **Phase 1: CAN Communication Tests** (приоритет 1)
Создай `renode/tests/can_communication.robot`:
1. ✅ Create CAN Hub in Renode
2. ✅ Send iRPC command (SetVelocity)
3. ✅ Verify UART log: "Received CAN message"
4. ✅ Verify response on CAN bus
5. ✅ Test timeout handling (no response)
6. ✅ Test invalid node ID
7. ✅ Test malformed message

### **Phase 2: FOC Control Tests** (приоритет 2)
Создай `renode/tests/foc_control.robot`:
1. ✅ FOC task startup
2. ✅ ADC calibration (zero offsets)
3. ✅ Simulate current readings (mock ADC values)
4. ✅ Simulate encoder position (mock encoder)
5. ✅ Send SetVelocity command
6. ✅ Verify PWM output changes
7. ✅ Verify control loop iteration count
8. ✅ Test state machine transitions (Idle → Running)

### **Phase 3: Safety & Fault Tests** (приоритет 3)
Создай `renode/tests/safety.robot`:
1. ✅ Overcurrent detection (inject high ADC values)
2. ✅ Emergency stop command
3. ✅ PWM disable on fault
4. ✅ CAN watchdog timeout
5. ✅ Fault recovery sequence

### **Phase 4: Integration Tests** (приоритет 4)
Создай `renode/tests/integration.robot`:
1. ✅ Full system startup
2. ✅ CAN command → FOC response → PWM output (end-to-end)
3. ✅ Velocity tracking (ramp up/down)
4. ✅ Position tracking (step response)
5. ✅ Load disturbance rejection

---

## 💡 Подсказки

### **1. Renode Python Peripherals:**
Для мокирования энкодера/ADC создай в `.repl`:
```python
encoder: Python.PythonPeripheral @ sysbus <0x50000000, +0x1000>
    size: 0x1000
    initable: true
    script: "
# Synthetic encoder position
position = 0

def read_position(offset):
    global position
    if offset == 0x00:  # Position register
        position = (position + 100) % 65536  # Increment
        return position
    return 0
    "
```

### **2. CAN Message Injection:**
```robot
*** Test Cases ***
Should Receive CAN Command
    Execute Command    sysbus.fdcan1 SendFrame 0x01 [0x10, 0x20, 0x30]
    Wait For Line On Uart    Received CAN message    timeout=2
```

### **3. State Verification:**
```robot
${state}=    Execute Command    cpu PC
Should Be Equal    ${state}    0x80001234    # Example address
```

### **4. Timing Tests:**
Используй Renode virtual time:
```robot
Execute Command    emulation RunFor "00:00:01.000"  # 1 second
```

---

## ✅ Критерии успеха

### **Минимум:**
- [ ] 15+ тестов проходят (5 базовых + 10 новых)
- [ ] CAN communication полностью покрыт
- [ ] FOC state machine протестирован
- [ ] Safety mechanisms проверены

### **Идеал:**
- [ ] 30+ тестов проходят
- [ ] 100% coverage критического функционала
- [ ] Performance tests (latency, throughput)
- [ ] Integration tests (end-to-end)
- [ ] Regression suite для CI/CD

---

## 🚀 Начни с этого

**Шаг 1:** Изучи существующие тесты:
```bash
cat renode/tests/basic_startup.robot
```

**Шаг 2:** Создай `can_communication.robot` с 1 простым тестом:
```robot
Should Create CAN Hub
    Execute Command    emulation CreateCANHub "canHub"
    Execute Command    connector Connect sysbus.fdcan1 canHub
```

**Шаг 3:** Постепенно добавляй сложность:
- CAN frame transmission
- iRPC message decoding
- Command handling
- Response verification

**Шаг 4:** Переходи к FOC тестам после того как CAN работает.

---

## 📞 Важные команды

### **Запуск тестов:**
```bash
# Все тесты
docker compose run --rm renode bash -c "
  cargo build --release --features renode-mock && 
  renode-test renode/tests/
"

# Один файл
renode-test renode/tests/can_communication.robot
```

### **Debug:**
```bash
# Interactive Renode
renode renode/stm32g431_foc.resc

# Monitor commands
(monitor) sysbus.fdcan1 SendFrame 0x01 [0x10, 0x20]
(monitor) sysbus.usart1
```

---

## 🎯 ТВОЯ ЦЕЛЬ

**Создай production-ready test suite для embedded Rust motor controller!**

- ✅ Comprehensive coverage
- ✅ Realistic scenarios
- ✅ Edge cases handled
- ✅ CI/CD ready
- ✅ Easy to maintain

**Давай сделаем это мощно!** 💪🚀

---

## 📌 Ключевые файлы для начала работы

1. `renode/tests/basic_startup.robot` - Пример структуры тестов
2. `renode/stm32g431cb.repl` - Платформа и peripherals
3. `src/firmware/tasks/foc.rs` - FOC controller (что тестируем)
4. `src/firmware/tasks/can_comm.rs` - CAN task (что тестируем)
5. `docs/README_RENODE.md` - Полная документация

**Погнали тестировать!** 🎯
