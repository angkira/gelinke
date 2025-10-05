# 🐳 Docker Setup для Renode Эмуляции

Самый простой способ запустить Renode - через Docker. Не нужно устанавливать зависимости на хост.

## ⚡ Быстрый старт (3 команды)

```bash
# 1. Собрать Docker image (один раз, ~5 минут)
./renode-docker.sh build

# 2. Собрать firmware
./renode-docker.sh firmware

# 3. Запустить эмуляцию
./renode-docker.sh run
```

**Всё!** Renode запустится с вашим firmware.

---

## 🎯 Преимущества Docker подхода

### ✅ Что упрощается:

1. **Нет установки зависимостей**
   - Не нужен Renode на хосте
   - Не нужен mono-complete и GTK# (~10+ пакетов!)
   - Не нужен Rust toolchain (опционально)
   - Всё в контейнере

2. **Воспроизводимое окружение**
   - Одинаково работает везде
   - Версия Renode зафиксирована
   - Нет конфликтов с системными пакетами

3. **Простая очистка**
   ```bash
   docker compose down -v  # Удалить всё
   ```

4. **Изоляция**
   - Не засоряет систему
   - Можно иметь несколько версий
   - Легко переключаться между проектами

### ❌ Vs нативная установка:

| Критерий | Docker | Нативно |
|----------|--------|---------|
| Setup время | ~5 мин build | ~2 мин install |
| Overhead | +100-200 MB RAM | Нет |
| GUI Renode | Через X11 | Напрямую |
| GDB debug | Порт 3333 | Порт 3333 |
| Скорость | ~95% нативной | 100% |

**Рекомендация:** Docker для 90% случаев, нативно только если нужна максимальная скорость.

---

## 📦 Что внутри

### Dockerfile.renode:
- Ubuntu 22.04
- Renode 1.15.0
- Rust toolchain + ARM target
- GDB для отладки
- X11 support для GUI

### docker-compose.yml:
- Service `renode` - основной контейнер
- Service `renode-test` - только для тестов
- Volume caching для быстрой сборки
- Port forwarding для GDB (3333)

### renode-docker.sh:
Wrapper скрипт для удобства.

---

## 🎮 Команды

### Базовые:

```bash
./renode-docker.sh build          # Собрать Docker image
./renode-docker.sh firmware       # Собрать firmware (debug)
./renode-docker.sh firmware-release  # Собрать firmware (release)
./renode-docker.sh run            # Запустить эмуляцию (GUI)
./renode-docker.sh headless       # Запустить headless
./renode-docker.sh test           # Запустить тесты
```

### Отладка:

```bash
./renode-docker.sh shell          # Bash в контейнере
./renode-docker.sh gdb            # GDB debugging
./renode-docker.sh logs           # Показать логи
```

### Управление:

```bash
./renode-docker.sh clean          # Очистить build
./renode-docker.sh rebuild        # Полная пересборка
./renode-docker.sh stop           # Остановить контейнеры
```

---

## 🚀 Типичный workflow

### Первый запуск:

```bash
# 1. Собрать image (один раз)
./renode-docker.sh build

# 2. Собрать firmware
./renode-docker.sh firmware

# 3. Запустить
./renode-docker.sh run
```

### Ежедневная работа:

```bash
# 1. Изменить код
vim src/...

# 2. Пересобрать и запустить
./renode-docker.sh firmware && ./renode-docker.sh run

# Или сразу с тестами
./renode-docker.sh firmware && ./renode-docker.sh test
```

### Быстрая проверка:

```bash
# Одной командой: build + test
./renode-docker.sh firmware && ./renode-docker.sh test
```

---

## 🎨 GUI vs Headless

### GUI Mode (с X11 forwarding):

```bash
./renode-docker.sh run

# Linux: работает из коробки
# macOS: нужен XQuartz
# Windows: нужен VcXsrv или WSL2 с X11
```

### Headless Mode (только UART логи):

```bash
./renode-docker.sh headless

# Увидишь:
# CLN17 v2.0 Joint Firmware
# Target: STM32G431CB @ 170 MHz
# System heartbeat: 1 sec
```

**Для автоматических тестов используется headless.**

---

## 🐛 GDB Debugging через Docker

### Вариант 1: Внутри контейнера

```bash
# Terminal 1: Запустить Renode с GDB server
./renode-docker.sh shell
# внутри контейнера:
renode renode/stm32g431_foc.resc

# Terminal 2: GDB в том же контейнере
docker compose exec renode bash
gdb-multiarch target/thumbv7em-none-eabihf/debug/joint_firmware
(gdb) target remote localhost:3333
(gdb) load
(gdb) break main
(gdb) continue
```

### Вариант 2: GDB на хосте

```bash
# Terminal 1: Renode в Docker
./renode-docker.sh run

# Terminal 2: GDB на хосте (если установлен)
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/joint_firmware
(gdb) target remote localhost:3333
(gdb) load
(gdb) continue
```

Порт 3333 пробрасывается автоматически.

---

## 🔧 Кастомизация

### Изменить версию Renode:

Отредактировать `Dockerfile.renode`:
```dockerfile
ARG RENODE_VERSION=1.15.0  # <- изменить здесь
```

Пересобрать:
```bash
./renode-docker.sh rebuild
```

### Добавить инструменты:

```dockerfile
# В Dockerfile.renode
RUN apt-get install -y \
    your-tool \
    another-tool
```

### Использовать свой Rust toolchain:

Если хотите использовать Rust с хоста:
```yaml
# В docker-compose.yml
volumes:
  - ~/.cargo:/root/.cargo  # Монтировать локальный cargo
```

---

## 📊 Производительность

### Размеры:

```
Docker image:     ~2.5 GB (с Rust toolchain)
Cargo cache:      ~500 MB
Target cache:     ~1 GB
Итого:            ~4 GB

Без Docker:       ~3 GB (Renode + Rust на хосте)
```

**Разница:** ~1 GB (кэши Docker)

### Скорость:

```
Build firmware:   ~95% скорости нативной сборки
Renode эмуляция:  ~98% скорости нативной
Тесты:            ~95% скорости

Overhead:         +100-200 MB RAM
```

**Вывод:** Практически незаметно для разработки.

---

## 🛠️ Troubleshooting

### "Cannot connect to Docker daemon"

```bash
# Запустить Docker
sudo systemctl start docker

# Добавить себя в группу docker
sudo usermod -aG docker $USER
# Перелогиниться
```

### "X11 connection rejected"

```bash
# Linux: разрешить X11
xhost +local:docker

# macOS: установить XQuartz
brew install --cask xquartz
```

### "Build очень медленный"

```bash
# Использовать кэши
docker compose run --rm renode cargo build

# Volumes уже настроены в docker-compose.yml
```

### "Порт 3333 занят"

```bash
# Проверить что занимает
lsof -i :3333

# Остановить старый Renode
./renode-docker.sh stop
```

---

## 🔄 Обновление

### Обновить Renode версию:

```bash
# Изменить RENODE_VERSION в Dockerfile.renode
vim Dockerfile.renode

# Пересобрать
./renode-docker.sh rebuild
```

### Обновить Rust:

```bash
./renode-docker.sh shell
# внутри:
rustup update
```

---

## 🗑️ Очистка

### Удалить кэши:

```bash
docker compose down -v  # Удалить volumes
```

### Удалить image:

```bash
docker rmi joint-firmware-renode:latest
```

### Полная очистка Docker:

```bash
docker system prune -a --volumes
# ВНИМАНИЕ: удалит ВСЁ в Docker, не только этот проект!
```

---

## 🌐 Кросс-платформенность

### Linux:
✅ Работает из коробки

### macOS:
✅ Работает с Docker Desktop  
⚠️ GUI требует XQuartz

### Windows:
✅ Работает с Docker Desktop + WSL2  
⚠️ GUI требует VcXsrv или WSL2 с X11

---

## 📚 Альтернативы

### Если Docker не подходит:

1. **Нативная установка** - см. `LOCAL_TESTING.md`
2. **Dev Container** - для VS Code (можно добавить `.devcontainer`)
3. **GitHub Codespaces** - облачная разработка

---

## ✅ Резюме

### Docker - лучший выбор если:
- ✅ Не хочешь устанавливать зависимости
- ✅ Работаешь на разных машинах
- ✅ Нужна изоляция от системы
- ✅ Работаешь в команде (единое окружение)

### Нативная установка - если:
- ✅ Нужна максимальная производительность
- ✅ Активно используешь GUI Renode
- ✅ Не хочешь Docker overhead

---

**Рекомендация:** Попробуй Docker - если понравится, оставь. Если нет - используй нативную установку из `LOCAL_TESTING.md`.

---

## 🚀 Quick Reference

```bash
# Первый запуск
./renode-docker.sh build && ./renode-docker.sh firmware && ./renode-docker.sh run

# Разработка
vim src/... && ./renode-docker.sh firmware && ./renode-docker.sh test

# Отладка
./renode-docker.sh shell

# Очистка
./renode-docker.sh clean
```

---

**Создано:** 2025-10-03  
**Проект:** STM32G431CB FOC Controller  
**Docker Image:** Ubuntu 22.04 + Renode 1.15.0

