# 🚀 Быстрый старт эмуляции STM32G431CB

## TL;DR - За 3 минуты

```bash
# 1. Установить Renode
wget https://github.com/renode/renode/releases/download/v1.15.0/renode_1.15.0_amd64.deb
sudo apt-get install -y mono-complete
sudo dpkg -i renode_1.15.0_amd64.deb

# 2. Собрать firmware
cargo build --target thumbv7em-none-eabihf

# 3. Запустить эмуляцию
renode renode/stm32g431_foc.resc

# ИЛИ автоматизированные тесты
renode-test renode/tests/*.robot
```

## 📦 Что создано

```
renode/
├── stm32g431cb.repl           # Platform description (железо)
├── stm32g431_foc.resc         # Startup script (конфиг)
├── README.md                  # Полная документация
├── manual_test.sh             # Скрипт для быстрого тестирования
└── tests/
    ├── basic_startup.robot    # Тесты загрузки
    ├── can_communication.robot # Тесты CAN-FD
    └── foc_control.robot      # Тесты FOC управления
```

## ⚡ Быстрые команды

### Интерактивная эмуляция:
```bash
./renode/manual_test.sh interactive
```

### Автоматизированные тесты:
```bash
./renode/manual_test.sh all      # Все тесты
./renode/manual_test.sh basic    # Только startup
./renode/manual_test.sh can      # Только CAN
./renode/manual_test.sh foc      # Только FOC
```

### CI/CD (опционально):
GitHub Actions workflow готов: `.github/workflows/renode-ci.yml.example`

Если захотите автоматические тесты на каждый push:
```bash
mv .github/workflows/renode-ci.yml.example .github/workflows/renode-ci.yml
```

**Пока работайте локально - это быстрее!** См. `LOCAL_TESTING.md`

## 🎯 Что эмулируется

| Периферия | Статус | Описание |
|-----------|--------|----------|
| CPU | ✅ | Cortex-M4F @ 170 MHz с FPU |
| TIM1 | ✅ | 3-фазный PWM 20 kHz |
| ADC1 | ✅ | Токи фаз + DMA |
| SPI1 | ✅ | TLE5012B энкодер |
| FDCAN1 | ✅ | CAN-FD + multi-device hub |
| CORDIC | ✅ | Park/Clarke трансформации |
| FMAC | ✅ | PI контроллеры |
| USART1 | ✅ | Debug telemetry |
| GPIO | ✅ | Все порты A/B/C |

## 🔥 Преимущества

### До (без эмуляции):
- ❌ Ждать производство железа (недели/месяцы)
- ❌ Тестировать только на реальном железе
- ❌ Длинный цикл разработки
- ❌ Риск повредить железо при отладке

### После (с Renode):
- ✅ Разработка и тестирование **сейчас**
- ✅ Автоматизированные тесты в CI/CD
- ✅ Симуляция физики мотора
- ✅ Multi-device сети без проводов
- ✅ Deterministic debugging
- ✅ Infinite hardware reset :)

## 🎮 Примеры использования

### 1. Проверить загрузку firmware:
```bash
renode renode/stm32g431_foc.resc

# В Renode console увидите:
# ╔═══════════════════════════════════════════════════════════════════════╗
# ║ STM32G431CB FOC Motor Controller - Renode Emulation                  ║
# ...
# CLN17 v2.0 Joint Firmware
# Target: STM32G431CB @ 170 MHz
# System Ready
# System heartbeat: 1 sec
```

### 2. Отладка CAN протокола:
```bash
# В Renode console:
(monitor) sysbus.fdcan1 Log

# Отправить тестовый CAN frame
(monitor) sysbus.fdcan1 SendFrame 0x10 0x01 0x02 0x03

# Увидите обработку в firmware
```

### 3. Проверить PWM:
```bash
(monitor) sysbus.tim1
(monitor) sysbus.tim1 Limit  # Должно быть 8500 для 20kHz
```

### 4. Симулировать вращение мотора:
```bash
# Изменить скорость вращения
(monitor) sysbus.currentSensorA Frequency 200  # 200 Hz

# Изменить позицию энкодера
(monitor) sysbus.encoder Position 16384  # Половина оборота
```

## 📊 Мониторинг в реальном времени

```bash
# В Renode console:
(monitor) showAnalyzer sysbus.usart1    # UART лог
(monitor) sysbus.fdcan1 Log             # CAN трафик
(monitor) logLevel 0 sysbus.tim1        # PWM updates

# Запустить на N секунд
(monitor) emulation RunFor "00:00:10"

# Посмотреть состояние CPU
(monitor) cpu
(monitor) cpu PC
(monitor) cpu SP
```

## 🧪 Разработка новых тестов

Создать `renode/tests/my_test.robot`:

```robotframework
*** Settings ***
Resource          ${RENODEKEYWORDS}

*** Test Cases ***
My Test
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @renode/stm32g431cb.repl
    Execute Command         sysbus LoadELF @target/thumbv7em-none-eabihf/debug/joint_firmware
    Create Terminal Tester  sysbus.usart1
    Start Emulation
    
    Wait For Line On Uart   System Ready    timeout=5
```

Запустить:
```bash
renode-test renode/tests/my_test.robot
```

## 🐛 Troubleshooting

### "Renode not found":
```bash
which renode
# Если нет - установить с releases
```

### "Permission denied" для manual_test.sh:
```bash
chmod +x renode/manual_test.sh
```

### Firmware не загружается:
```bash
# Проверить ELF файл существует
ls -lh target/thumbv7em-none-eabihf/debug/joint_firmware

# Пересобрать
cargo clean
cargo build --target thumbv7em-none-eabihf
```

### Тесты падают:
```bash
# Посмотреть детальный лог
renode-test --show-log renode/tests/basic_startup.robot
```

## 📚 Дополнительно

- **Полная документация**: `renode/README.md`
- **Renode docs**: https://renode.readthedocs.io/
- **Robot Framework**: https://robotframework.org/

## 🎓 Следующие шаги

1. **Сейчас**: Запустите `./renode/manual_test.sh` и убедитесь что firmware загружается
2. **Разработка**: Используйте эмуляцию для тестирования новых фич
3. **CI/CD**: Пушьте в GitHub - автоматически запустятся тесты
4. **Отладка**: Используйте GDB + Renode для step-by-step debugging
5. **Интеграция**: Тестируйте multi-device CAN сети

---

**Разработано для**: STM32G431CB FOC Motor Controller  
**Дата**: 2025-10-03  
**Автор**: AI Assistant + You 🚀

