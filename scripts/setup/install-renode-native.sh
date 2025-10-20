#!/bin/bash
################################################################################
# Скрипт установки Renode нативно (без Docker)
# Устанавливает все необходимые зависимости автоматически
################################################################################

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

RENODE_VERSION="1.15.0"

echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Установка Renode ${RENODE_VERSION} (нативно)                                 ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Проверка что запущено на Linux
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo -e "${RED}ERROR: Этот скрипт работает только на Linux${NC}"
    echo "Для других ОС используйте Docker: ./renode-docker.sh"
    exit 1
fi

# Проверка прав sudo
if ! sudo -n true 2>/dev/null; then
    echo -e "${YELLOW}Потребуются права sudo для установки${NC}"
fi

echo -e "${BLUE}[1/4] Обновление списка пакетов...${NC}"
sudo apt-get update -qq

echo -e "${BLUE}[2/4] Установка зависимостей...${NC}"
sudo apt-get install -y -qq \
    wget \
    mono-complete \
    gtk-sharp2 \
    gtk-sharp2-gapi \
    libglade2.0-cil-dev \
    libglib2.0-cil-dev \
    libgtk2.0-cil-dev \
    screen \
    policykit-1 2>&1 | grep -v "^Selecting\|^Preparing\|^Unpacking\|^Setting up" || true

echo -e "${GREEN}✓ Зависимости установлены${NC}"

echo -e "${BLUE}[3/4] Загрузка Renode ${RENODE_VERSION}...${NC}"
if [ ! -f "renode_${RENODE_VERSION}_amd64.deb" ]; then
    wget -q --show-progress \
        https://github.com/renode/renode/releases/download/v${RENODE_VERSION}/renode_${RENODE_VERSION}_amd64.deb
else
    echo -e "${YELLOW}Файл уже загружен, пропускаю${NC}"
fi

echo -e "${BLUE}[4/4] Установка Renode...${NC}"
sudo dpkg -i renode_${RENODE_VERSION}_amd64.deb 2>&1 || true
sudo apt-get install -f -y -qq 2>&1 | grep -v "^Selecting\|^Preparing\|^Unpacking\|^Setting up" || true

echo ""
echo -e "${GREEN}✓ Установка завершена!${NC}"
echo ""

# Проверка
if command -v renode &> /dev/null; then
    INSTALLED_VERSION=$(renode --version 2>&1 | head -n1 || echo "unknown")
    echo -e "${GREEN}Renode установлен:${NC} $INSTALLED_VERSION"
    echo -e "${GREEN}Путь:${NC} $(which renode)"
    echo -e "${GREEN}renode-test:${NC} $(which renode-test)"
    echo ""
    echo -e "${BLUE}Запустить:${NC}"
    echo "  cargo build --target thumbv7em-none-eabihf"
    echo "  renode renode/stm32g431_foc.resc"
    echo ""
    echo -e "${BLUE}Тесты:${NC}"
    echo "  ./renode/manual_test.sh all"
else
    echo -e "${RED}ERROR: Renode не установлен${NC}"
    echo "Попробуйте установить вручную или используйте Docker"
    exit 1
fi

# Cleanup
echo -e "${YELLOW}Очистить установочный файл? (y/n)${NC}"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    rm -f renode_${RENODE_VERSION}_amd64.deb
    echo -e "${GREEN}✓ Очищено${NC}"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Готово! Renode установлен и готов к использованию                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════╝${NC}"

