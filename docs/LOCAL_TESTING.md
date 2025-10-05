# 🚀 Локальное тестирование с Renode

Быстрый гайд для локальной работы без GitHub/CI.

## ⚡ Установка (один раз)

### Вариант 1: Docker (рекомендуется) 🐳

```bash
# Всё в одном контейнере
./renode-docker.sh build
```

**См. `DOCKER_SETUP.md` для деталей.**

### Вариант 2: Нативная установка

**Автоматически (рекомендуется):**
```bash
./install-renode-native.sh
```

**Вручную:**
```bash
# 1. Установить зависимости
sudo apt-get update
sudo apt-get install -y \
    mono-complete \
    gtk-sharp2 \
    gtk-sharp2-gapi \
    libglade2.0-cil-dev \
    libglib2.0-cil-dev \
    libgtk2.0-cil-dev \
    screen \
    policykit-1

# 2. Скачать и установить Renode
wget https://github.com/renode/renode/releases/download/v1.15.0/renode_1.15.0_amd64.deb
sudo dpkg -i renode_1.15.0_amd64.deb

# 3. Если были ошибки зависимостей - исправить
sudo apt-get install -f -y

# 4. Проверить
renode --version
which renode-test
```

**Зависимости которые нужны:**
- `mono-complete` - Mono runtime
- `gtk-sharp2*` - GTK# биндинги для GUI
- `libglade/libglib/libgtk*-cil-dev` - GTK библиотеки
- `screen` - Terminal multiplexer
- `policykit-1` - PolicyKit для привилегий

## 🎮 Ежедневная работа

### 1. Разработка и быстрая проверка

**Docker:**
```bash
vim src/...
./renode-docker.sh firmware && ./renode-docker.sh run
```

**Нативно:**
```bash
vim src/...
cargo build --target thumbv7em-none-eabihf
renode renode/stm32g431_foc.resc
```

В Renode console увидите:
```
╔═══════════════════════════════════════════════════════════════════════╗
║ STM32G431CB FOC Motor Controller - Renode Emulation                  ║
...
CLN17 v2.0 Joint Firmware
Target: STM32G431CB @ 170 MHz
System heartbeat: 1 sec
System heartbeat: 2 sec
```

**Ctrl+C** для выхода.

### 2. Автоматические тесты

```bash
# Быстрый тест загрузки
./renode/manual_test.sh basic

# Все тесты
./renode/manual_test.sh all

# Или напрямую
renode-test renode/tests/basic_startup.robot
renode-test renode/tests/can_communication.robot
renode-test renode/tests/foc_control.robot
```

### 3. Build + Test (один скрипт)

```bash
./renode/manual_test.sh build-test
```

## 🔧 Интерактивная отладка

### Базовая эмуляция:

```bash
renode renode/stm32g431_foc.resc

# В Renode console:
(monitor) showAnalyzer sysbus.usart1    # Показать UART логи
(monitor) pause                         # Пауза
(monitor) start                         # Продолжить
(monitor) cpu PC                        # Program Counter
(monitor) quit                          # Выход
```

### С GDB:

```bash
# Terminal 1: Renode
renode renode/stm32g431_foc.resc

# Terminal 2: GDB
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/joint_firmware
(gdb) target remote :3333
(gdb) load
(gdb) break main
(gdb) continue
(gdb) next
(gdb) print some_variable
```

## 📊 Мониторинг

### UART логи (defmt):
```bash
# В Renode console:
(monitor) showAnalyzer sysbus.usart1
```

### CAN трафик:
```bash
(monitor) sysbus.fdcan1 Log

# Отправить тестовый frame
(monitor) sysbus.fdcan1 SendFrame 0x10 0x01 0x02 0x03
```

### PWM состояние:
```bash
(monitor) sysbus.tim1
(monitor) sysbus.tim1 Limit  # Должно быть 8500 для 20kHz
```

### Симуляция мотора:
```bash
# Изменить скорость вращения
(monitor) sysbus.currentSensorA Frequency 200  # Hz

# Изменить позицию энкодера
(monitor) sysbus.encoder Position 16384  # Половина оборота
```

## 🎯 Типичный workflow

```bash
# 1. Написать код
vim src/firmware/...

# 2. Unit тесты (быстро)
cargo test --lib

# 3. Собрать для ARM
cargo build --target thumbv7em-none-eabihf

# 4. Интерактивная проверка
renode renode/stm32g431_foc.resc
# Смотрим логи, проверяем работу

# 5. Автоматические тесты
./renode/manual_test.sh all

# 6. Если всё ОК - commit
git add .
git commit -m "feat: новая фича"
```

## 🧪 Кастомизация тестов

### Изменить параметры симуляции:

Редактировать `renode/stm32g431cb.repl`:

```python
# Параметры мотора
currentSensorA:
    frequency: 100      # Hz - скорость вращения
    amplitude: 0.5      # Ампер - ток
    
encoder:
    resolution: 32768   # 15-bit TLE5012B
    initialPosition: 0  # Начальная позиция
```

### Создать свой тест:

Создать `renode/tests/my_test.robot`:

```robotframework
*** Settings ***
Resource          ${RENODEKEYWORDS}

*** Test Cases ***
My Custom Test
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @renode/stm32g431cb.repl
    Execute Command         sysbus LoadELF @target/thumbv7em-none-eabihf/debug/joint_firmware
    Create Terminal Tester  sysbus.usart1
    Start Emulation
    
    Wait For Line On Uart   System Ready    timeout=5
    
    # Ваши проверки здесь
```

Запустить:
```bash
renode-test renode/tests/my_test.robot
```

## 📋 Скрипт manual_test.sh

```bash
./renode/manual_test.sh interactive   # Интерактивная сессия (по умолчанию)
./renode/manual_test.sh basic         # Тесты загрузки
./renode/manual_test.sh can           # Тесты CAN
./renode/manual_test.sh foc           # Тесты FOC
./renode/manual_test.sh all           # Все тесты
./renode/manual_test.sh build-test    # Build + тест
```

## 🐛 Troubleshooting

### "renode: command not found"
```bash
which renode
# Если нет - установить заново или добавить в PATH
export PATH=$PATH:/opt/renode
```

### "ELF not found"
```bash
# Пересобрать
cargo build --target thumbv7em-none-eabihf

# Проверить файл
ls -lh target/thumbv7em-none-eabihf/debug/joint_firmware
```

### "Permission denied: manual_test.sh"
```bash
chmod +x renode/manual_test.sh
```

### Тесты падают
```bash
# Детальный лог
renode-test --show-log renode/tests/basic_startup.robot

# Или через Renode с verbose
renode --console renode/stm32g431_foc.resc
```

### Firmware не грузится
```bash
renode renode/stm32g431_foc.resc

# В Renode console:
(monitor) sysbus WhatIsLoaded        # Проверить загружен ли ELF
(monitor) cpu PC                     # Должен быть 0x0800xxxx (flash)
(monitor) showAnalyzer sysbus.usart1 # Смотреть логи
```

## 📚 Шпаргалки

### Основные команды:
См. `renode/CHEATSHEET.md`

### Полная документация:
См. `renode/README.md`

### Быстрый старт:
См. `EMULATION_QUICKSTART.md`

## 🚀 Преимущества локальной работы

✅ Быстрая обратная связь (секунды)  
✅ Не нужен интернет  
✅ Нет ожидания CI/CD  
✅ Полный контроль  
✅ GDB debugging  
✅ Симуляция различных сценариев  

---

## 📦 Когда захотите CI/CD

Файл `.github/workflows/renode-ci.yml.example` готов к использованию:

```bash
# Включить GitHub Actions
mv .github/workflows/renode-ci.yml.example .github/workflows/renode-ci.yml
git add .github/workflows/renode-ci.yml
git commit -m "ci: enable Renode tests"
git push
```

Но пока работайте локально - это быстрее! ⚡

---

**Проект:** STM32G431CB FOC Controller  
**Дата:** 2025-10-03  
**Эмуляция:** Renode 1.15.0

