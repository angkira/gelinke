# Renode Emulation for STM32G431CB FOC Controller

Эмуляция firmware на STM32G431CB без физического железа используя [Renode](https://renode.io/).

## 🎯 Что эмулируется

### Основные периферийные устройства:
- ✅ **Cortex-M4F** @ 170 MHz с FPU и NVIC
- ✅ **TIM1** - 3-фазный PWM с complementary outputs (20 kHz)
- ✅ **ADC1** - Датчики токов фаз A и B с DMA
- ✅ **SPI1** - Энкодер TLE5012B (15-bit)
- ✅ **FDCAN1** - CAN-FD коммуникация
- ✅ **CORDIC** - Аппаратный акселератор для Park/Clarke
- ✅ **FMAC** - Аппаратный акселератор для PI контроллеров
- ✅ **USART1** - Debug телеметрия через UART
- ✅ **GPIO** - Все порты (A, B, C)
- ✅ **Flash** (128KB) + **SRAM** (32KB)

### Симуляция мотора:
- 📊 Синусоидальные токи фаз (имитация реального мотора)
- 🔄 Энкодер со смещением фазы 120° между обмотками
- ⚡ CAN-FD Hub для multi-device тестирования

## 📦 Установка Renode

### Ubuntu/Debian:
```bash
wget https://github.com/renode/renode/releases/download/v1.15.0/renode_1.15.0_amd64.deb
sudo apt-get install -y mono-complete
sudo dpkg -i renode_1.15.0_amd64.deb
```

### Другие ОС:
Скачать с https://github.com/renode/renode/releases

## 🚀 Быстрый старт

### 1. Собрать firmware:
```bash
cd /home/angkira/Project/software/joint_firmware
cargo build --target thumbv7em-none-eabihf
```

### 2. Запустить эмуляцию:
```bash
renode renode/stm32g431_foc.resc
```

Renode загрузит firmware и запустит эмуляцию. В консоли увидите:

```
╔═══════════════════════════════════════════════════════════════════════╗
║ STM32G431CB FOC Motor Controller - Renode Emulation                  ║
╟───────────────────────────────────────────────────────────────────────╢
║ Target:      STM32G431CB @ 170 MHz                                   ║
║ Framework:   Embassy Async Runtime                                   ║
║ Protocol:    iRPC over CAN-FD                                        ║
...
```

### 3. Проверить работу:

В Renode console:
```
(monitor) sysbus.usart1
# Показать UART вывод (defmt логи)

(monitor) sysbus.fdcan1 Log
# Показать CAN трафик

(monitor) sysbus.tim1
# Проверить состояние PWM таймера

(monitor) cpu PC
# Посмотреть текущий Program Counter
```

## 🧪 Автоматизированное тестирование

### Запуск Robot Framework тестов:

```bash
# Все тесты
renode-test renode/tests/*.robot

# Конкретный тест
renode-test renode/tests/basic_startup.robot
```

### Структура тестов:

```
renode/tests/
├── basic_startup.robot       # Базовая загрузка firmware
├── can_communication.robot   # CAN-FD и iRPC протокол
└── foc_control.robot         # FOC управление мотором
```

### Пример вывода:
```
==============================================================================
Basic Startup
==============================================================================
Should Boot And Show Banner                                          | PASS |
Should Initialize System                                             | PASS |
Should Start Heartbeat                                               | PASS |
Should Initialize PWM                                                | PASS |
Should Initialize CAN                                                | PASS |
==============================================================================
Basic Startup                                                        | PASS |
5 tests, 5 passed, 0 failed
```

## 🎮 Интерактивная отладка

### GDB Debugging:

```bash
# Terminal 1: Запустить Renode с GDB сервером
renode renode/stm32g431_foc.resc

# Terminal 2: Подключить GDB
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/joint_firmware
(gdb) target remote :3333
(gdb) load
(gdb) break main
(gdb) continue
```

### RTT для defmt:

Renode поддерживает RTT (Real-Time Transfer) для вывода defmt логов:

```bash
# В Renode console
(monitor) machine CreateRttChannel "defmt" 0
(monitor) showAnalyzer sysbus.rtt
```

## 📊 Симуляция физики мотора

Platform description (`stm32g431cb.repl`) включает симуляцию мотора:

```python
# Токи фаз как синусоиды
currentSensorA: Analog.SineWaveGenerator @ adc1 0
    frequency: 100        # 100 Hz вращение
    amplitude: 0.5        # ±0.5A
    offset: 1.65          # Mid-rail (3.3V / 2)

currentSensorB: Analog.SineWaveGenerator @ adc1 1
    frequency: 100
    amplitude: 0.5
    offset: 1.65
    phase: 120            # Фаза B сдвинута на 120°

# Энкодер с 15-bit разрешением
encoder: Sensors.RotaryEncoder @ spi1
    resolution: 32768     # TLE5012B
    initialPosition: 0
```

Можно изменить параметры в runtime:

```
(monitor) sysbus.currentSensorA Frequency 200
(monitor) sysbus.encoder Position 16384
```

## 🔗 CAN-FD Multi-Device тестирование

Создать несколько нод на одной шине:

```python
# Renode script
mach create "node1"
machine LoadPlatformDescription @renode/stm32g431cb.repl
sysbus LoadELF @firmware1.elf

mach create "node2"
machine LoadPlatformDescription @renode/stm32g431cb.repl
sysbus LoadELF @firmware2.elf

# Соединить через CAN Hub
emulation CreateCANHub "motorBus"
connector Connect node1.sysbus.fdcan1 motorBus
connector Connect node2.sysbus.fdcan1 motorBus

start
```

## 🛠️ Кастомизация

### Изменить параметры мотора:

Отредактировать `stm32g431cb.repl`:

```python
encoder:
    resolution: 65536  # 16-bit вместо 15-bit
    
currentSensorA:
    frequency: 50      # Медленнее вращение
    amplitude: 1.0     # Больший ток
```

### Добавить новую периферию:

```python
// В .repl файл
tim2: Timers.STM32_Timer @ sysbus 0x40000000
    frequency: 170000000
    -> nvic@28
```

### Логирование:

```
# Включить verbose логи
(monitor) logLevel 0 sysbus.fdcan1
(monitor) logLevel 0 sysbus.tim1

# Отключить шумные логи
(monitor) logLevel 3 cpu
```

## 📋 CI/CD Integration (опционально)

Шаблон GitHub Actions готов: `.github/workflows/renode-ci.yml.example`

Если захотите включить автоматические тесты:
```bash
mv .github/workflows/renode-ci.yml.example .github/workflows/renode-ci.yml
git add .github/workflows/renode-ci.yml
git commit -m "ci: enable Renode tests"
```

**Рекомендация:** Сначала освойтесь с локальным тестированием.  
См. `../LOCAL_TESTING.md` для быстрого старта.

## 🐛 Troubleshooting

### Firmware не стартует:

```bash
# Проверить загрузился ли ELF
(monitor) sysbus WhatIsLoaded

# Проверить PC (должен быть в flash, 0x0800xxxx)
(monitor) cpu PC

# Сбросить
(monitor) runMacro $reset
```

### CAN не работает:

```bash
# Проверить FDCAN инициализирован
(monitor) sysbus.fdcan1

# Включить debug логи
(monitor) logLevel -1 sysbus.fdcan1
```

### PWM не генерируется:

```bash
# Проверить TIM1
(monitor) sysbus.tim1
(monitor) sysbus.tim1 Limit  # Должно быть 8500 для 20kHz
```

## 📚 Дополнительные ресурсы

- [Renode Documentation](https://renode.readthedocs.io/)
- [Renode STM32 Examples](https://github.com/renode/renode/tree/master/platforms/cpus)
- [Robot Framework Guide](https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html)
- [Embassy Framework](https://embassy.dev/)

## 🎓 Примеры команд

### Базовые:
```bash
start               # Запустить эмуляцию
pause               # Пауза
quit                # Выход

emulation RunFor "00:00:01"  # Запустить на 1 секунду
```

### Инспекция памяти:
```bash
sysbus ReadDoubleWord 0x20000000  # Прочитать SRAM
sysbus ReadDoubleWord 0x08000000  # Прочитать Flash
```

### Периферия:
```bash
sysbus WhatPeripheralsAreEnabled
peripherals
```

### Отладка:
```bash
cpu Step          # Выполнить одну инструкцию
cpu Step 100      # Выполнить 100 инструкций
cpu PC            # Program Counter
cpu SP            # Stack Pointer
```

---

**Создано для:** CLN17 v2.0 Joint Firmware  
**Target:** STM32G431CB @ 170 MHz  
**Framework:** Embassy + iRPC  
**Дата:** 2025-10-03

