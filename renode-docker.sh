#!/bin/bash
################################################################################
# Docker wrapper для Renode эмуляции
# Упрощенный интерфейс для работы с Renode в Docker
################################################################################

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Функция для красивого вывода
print_header() {
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║  STM32G431CB Renode Emulation - Docker Edition                    ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Проверка Docker
check_docker() {
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}ERROR: Docker не установлен${NC}"
        echo "Установи Docker: https://docs.docker.com/engine/install/"
        exit 1
    fi
    
    if ! command -v docker compose &> /dev/null; then
        echo -e "${RED}ERROR: docker compose не установлен${NC}"
        echo "Установи docker-compose: https://docs.docker.com/compose/install/"
        exit 1
    fi
}

# Build Docker image
build_image() {
    echo -e "${BLUE}Собираю Docker image...${NC}"
    docker compose build renode
    echo -e "${GREEN}✓ Image готов${NC}"
}

# Build firmware
build_firmware() {
    echo -e "${BLUE}Собираю firmware...${NC}"
    docker compose run --rm renode \
        cargo build --target thumbv7em-none-eabihf "$@"
    echo -e "${GREEN}✓ Firmware собран${NC}"
}

# Запустить интерактивную эмуляцию
run_interactive() {
    echo -e "${BLUE}Запускаю Renode (интерактивно)...${NC}"
    echo -e "${YELLOW}Используй X11 forwarding для GUI${NC}"
    echo ""
    
    # Разрешить X11 подключения (Linux)
    if [ -n "$DISPLAY" ]; then
        xhost +local:docker 2>/dev/null || true
    fi
    
    docker compose run --rm renode \
        renode renode/stm32g431_foc.resc
}

# Запустить headless с выводом UART
run_headless() {
    echo -e "${BLUE}Запускаю Renode (headless)...${NC}"
    docker compose run --rm renode bash -c "
        renode --disable-xwt \
               --console \
               renode/stm32g431_foc.resc
    "
}

# Запустить тесты
run_tests() {
    echo -e "${BLUE}Запускаю тесты...${NC}"
    docker compose run --rm renode-test
}

# Интерактивная shell в контейнере
run_shell() {
    echo -e "${BLUE}Запускаю bash в контейнере...${NC}"
    docker compose run --rm renode bash
}

# GDB debugging
run_gdb() {
    echo -e "${BLUE}Запускаю GDB debugging...${NC}"
    echo -e "${YELLOW}Terminal 1 (Renode):${NC} docker compose up renode"
    echo -e "${YELLOW}Terminal 2 (GDB):${NC} docker compose exec renode gdb-multiarch"
    echo ""
    docker compose up renode
}

# Очистка
clean() {
    echo -e "${BLUE}Очистка...${NC}"
    docker compose run --rm renode cargo clean
    echo -e "${GREEN}✓ Очищено${NC}"
}

# Полная пересборка
rebuild() {
    echo -e "${BLUE}Полная пересборка...${NC}"
    clean
    build_image
    build_firmware --release
    echo -e "${GREEN}✓ Пересборка завершена${NC}"
}

# Показать логи
show_logs() {
    docker compose logs -f renode
}

# Остановить все контейнеры
stop_all() {
    echo -e "${BLUE}Останавливаю контейнеры...${NC}"
    docker compose down
    echo -e "${GREEN}✓ Остановлено${NC}"
}

# Показать помощь
show_help() {
    print_header
    echo "Использование: $0 <команда>"
    echo ""
    echo "Команды:"
    echo "  ${GREEN}build${NC}          - Собрать Docker image"
    echo "  ${GREEN}firmware${NC}       - Собрать firmware (debug)"
    echo "  ${GREEN}firmware-release${NC} - Собрать firmware (release)"
    echo "  ${GREEN}run${NC}            - Запустить интерактивную эмуляцию (GUI)"
    echo "  ${GREEN}headless${NC}       - Запустить headless эмуляцию"
    echo "  ${GREEN}test${NC}           - Запустить все тесты"
    echo "  ${GREEN}shell${NC}          - Открыть bash в контейнере"
    echo "  ${GREEN}gdb${NC}            - Запустить с GDB debugging"
    echo "  ${GREEN}logs${NC}           - Показать логи"
    echo "  ${GREEN}clean${NC}          - Очистить build artifacts"
    echo "  ${GREEN}rebuild${NC}        - Полная пересборка"
    echo "  ${GREEN}stop${NC}           - Остановить все контейнеры"
    echo ""
    echo "Примеры:"
    echo "  $0 build           # Первый запуск"
    echo "  $0 firmware        # Собрать firmware"
    echo "  $0 run             # Запустить эмуляцию"
    echo "  $0 test            # Запустить тесты"
    echo ""
    echo "Быстрый старт:"
    echo "  ${GREEN}$0 build && $0 firmware && $0 run${NC}"
}

# Main
print_header
check_docker

COMMAND="${1:-help}"

case "$COMMAND" in
    build)
        build_image
        ;;
    firmware)
        build_firmware
        ;;
    firmware-release)
        build_firmware --release
        ;;
    run)
        run_interactive
        ;;
    headless)
        run_headless
        ;;
    test)
        run_tests
        ;;
    shell)
        run_shell
        ;;
    gdb)
        run_gdb
        ;;
    logs)
        show_logs
        ;;
    clean)
        clean
        ;;
    rebuild)
        rebuild
        ;;
    stop)
        stop_all
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo -e "${RED}Неизвестная команда: $COMMAND${NC}"
        echo ""
        show_help
        exit 1
        ;;
esac

