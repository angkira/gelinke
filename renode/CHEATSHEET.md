# Renode Quick Reference - Шпаргалка

## ⚡ Быстрые команды

### Установка (1 раз):
```bash
wget https://github.com/renode/renode/releases/download/v1.15.0/renode_1.15.0_amd64.deb
sudo apt install -y mono-complete && sudo dpkg -i renode_*.deb
```

### Основное использование:
```bash
# Интерактивная эмуляция
renode renode/stm32g431_foc.resc

# Автоматические тесты
renode-test renode/tests/*.robot
renode-test renode/tests/basic_startup.robot

# Через скрипт
./renode/manual_test.sh interactive
./renode/manual_test.sh all
./renode/manual_test.sh basic|can|foc
```

---

## 🎮 Команды Renode Console

### Основные:
```bash
start                              # Запустить эмуляцию
pause                              # Пауза
quit                               # Выход
help                               # Помощь
```

### Эмуляция:
```bash
emulation RunFor "00:00:01"        # Запустить на 1 сек
emulation RunFor "00:00:00.001"    # На 1 мс (10 FOC циклов)
```

### Периферия:
```bash
sysbus.usart1                      # Показать UART
showAnalyzer sysbus.usart1         # UART в окне

sysbus.fdcan1 Log                  # CAN трафик
sysbus.fdcan1 SendFrame 0x10 ...   # Отправить CAN frame

sysbus.tim1                        # Состояние PWM
sysbus.tim1 Limit                  # Max duty (8500 для 20kHz)

sysbus.adc1                        # Состояние ADC
sysbus.spi1                        # Состояние SPI

peripherals                        # Список всей периферии
sysbus WhatPeripheralsAreEnabled   # Активные устройства
```

### CPU & Память:
```bash
cpu PC                             # Program Counter
cpu SP                             # Stack Pointer
cpu Step                           # Выполнить 1 инструкцию
cpu Step 100                       # 100 инструкций

sysbus ReadDoubleWord 0x20000000   # Прочитать SRAM
sysbus ReadDoubleWord 0x08000000   # Прочитать Flash
sysbus WhatIsLoaded                # Что загружено
```

### Логирование:
```bash
logLevel 0 sysbus.fdcan1           # Verbose (все)
logLevel 1 sysbus.tim1             # Debug
logLevel 2 sysbus.adc1             # Info
logLevel 3 cpu                     # Warning (меньше шума)
logLevel -1 sysbus.fdcan1          # Максимум деталей
```

### Симуляция мотора:
```bash
# Изменить параметры в runtime
sysbus.currentSensorA Frequency 200       # Скорость вращения
sysbus.currentSensorA Amplitude 1.0       # Амплитуда тока
sysbus.encoder Position 16384             # Позиция энкодера (половина)
sysbus.encoder Position 0                 # Сброс позиции
```

### Отладка:
```bash
machine StartGdbServer 3333        # Запустить GDB server
machine Reset                      # Сброс MCU
runMacro $reset                    # Перезагрузить firmware
```

---

## 🐛 GDB Debugging

### Terminal 1 - Renode:
```bash
renode renode/stm32g431_foc.resc
# GDB server автоматически на порту 3333
```

### Terminal 2 - GDB:
```bash
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/joint_firmware
(gdb) target remote :3333
(gdb) load
(gdb) break main
(gdb) continue
(gdb) next
(gdb) step
(gdb) print variable
(gdb) backtrace
```

---

## 📊 Мониторинг

### UART Output (defmt логи):
```bash
# В Renode console:
showAnalyzer sysbus.usart1

# Или в терминале (если настроен file backend):
tail -f uart.log
```

### CAN Traffic:
```bash
# Логировать в файл
sysbus.fdcan1 CreateFileBackend @can_trace.log true
sysbus.fdcan1 Log

# Анализировать
cat can_trace.log
```

### PWM Waveforms:
```bash
sysbus.tim1
sysbus.tim1 Frequency      # Должна быть 20000 Hz
sysbus.tim1 Limit          # Max duty: 8500
```

---

## 🔧 Кастомизация Platform

### Изменить `renode/stm32g431cb.repl`:

```python
# Параметры симуляции мотора
currentSensorA: Analog.SineWaveGenerator @ adc1 0
    frequency: 100          # Hz - скорость вращения
    amplitude: 0.5          # Ампер - амплитуда тока
    offset: 1.65            # V - смещение (mid-rail)
    phase: 0                # Градусы

currentSensorB:
    phase: 120              # Сдвиг фазы B на 120°

encoder: Sensors.RotaryEncoder @ spi1
    resolution: 32768       # 15-bit (TLE5012B)
    initialPosition: 0      # Начальная позиция
```

---

## 🧪 Написание тестов

### Robot Framework template:

```robotframework
*** Settings ***
Resource          ${RENODEKEYWORDS}

*** Variables ***
${UART}           sysbus.usart1
${ELF}            target/thumbv7em-none-eabihf/debug/joint_firmware

*** Test Cases ***
My Test
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @renode/stm32g431cb.repl
    Execute Command         sysbus LoadELF @${ELF}
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   Expected text    timeout=5
    
    Execute Command         emulation RunFor "00:00:01"
    
    ${result}=              Execute Command  cpu PC
    Should Contain          ${result}        0x08
```

---

## 📋 Файлы проекта

```
renode/
├── stm32g431cb.repl           # ← Железо (периферия)
├── stm32g431_foc.resc         # ← Конфиг запуска
├── manual_test.sh             # ← Скрипт тестирования
├── README.md                  # ← Полная документация
├── CHEATSHEET.md              # ← Этот файл
└── tests/
    ├── basic_startup.robot    # ← Тест загрузки
    ├── can_communication.robot # ← Тест CAN
    └── foc_control.robot      # ← Тест FOC
```

---

## 💡 Частые сценарии

### Отладка загрузки firmware:
```bash
renode renode/stm32g431_foc.resc
(monitor) cpu PC                # Должен быть 0x0800xxxx
(monitor) sysbus WhatIsLoaded   # Проверить ELF загружен
(monitor) showAnalyzer sysbus.usart1  # Смотреть логи
```

### Тест CAN коммуникации:
```bash
renode renode/stm32g431_foc.resc
(monitor) sysbus.fdcan1 Log
(monitor) sysbus.fdcan1 SendFrame 0x10 0x01 0x02 0x03 0x04
(monitor) emulation RunFor "00:00:00.01"
# Проверить ответ в логах
```

### Симуляция работы мотора:
```bash
renode renode/stm32g431_foc.resc
(monitor) sysbus.currentSensorA Frequency 50   # Медленнее
(monitor) sysbus.encoder Position 8192         # 1/4 оборота
(monitor) emulation RunFor "00:00:10"
(monitor) showAnalyzer sysbus.usart1           # Смотреть телеметрию
```

### Multi-device CAN сеть:
```bash
# В .resc файле или console:
mach create "joint1"
machine LoadPlatformDescription @renode/stm32g431cb.repl
sysbus LoadELF @firmware.elf

mach create "joint2"  
machine LoadPlatformDescription @renode/stm32g431cb.repl
sysbus LoadELF @firmware.elf

emulation CreateCANHub "bus"
connector Connect joint1.sysbus.fdcan1 bus
connector Connect joint2.sysbus.fdcan1 bus

# Переключаться между нодами:
mach set "joint1"
mach set "joint2"

start
```

---

## 🚨 Troubleshooting

| Проблема | Команда для диагностики | Решение |
|----------|------------------------|---------|
| Не стартует | `cpu PC` | Должен быть 0x0800xxxx |
| Нет вывода UART | `showAnalyzer sysbus.usart1` | Проверить defmt-rtt |
| CAN не работает | `sysbus.fdcan1` | Включить `logLevel -1` |
| Firmware не загружается | `sysbus WhatIsLoaded` | Пересобрать cargo |
| Зависание | `pause`, потом `cpu PC` | Проверить бесконечный цикл |

---

## 📚 Дополнительно

- Полная документация: `renode/README.md`
- Быстрый старт: `../EMULATION_QUICKSTART.md`
- Обзор опций: `../docs/EMULATION_OPTIONS.md`
- Renode docs: https://renode.readthedocs.io/

---

**Версия:** 1.0  
**Дата:** 2025-10-03  
**Проект:** STM32G431CB FOC Controller

