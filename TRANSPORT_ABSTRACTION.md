# iRPC Transport Abstraction in Joint Firmware

## Концепция

Firmware полностью изолирован от деталей физической коммуникации (CAN-FD, USB, SPI).
Все взаимодействие происходит через `IrpcTransport` - gRPC-подобный слой абстракции.

## Архитектура

```
┌──────────────────────────────────────────────────────┐
│            Application Layer                         │
│  bridge.handle_message(&msg) -> response            │ ← Чистая бизнес-логика
└────────────────────┬─────────────────────────────────┘
                     │ Message (typed)
┌────────────────────▼─────────────────────────────────┐
│          IrpcTransport Layer                         │
│  • transport.send_message(&msg)                      │
│  • transport.receive_message()                       │ ← Transport abstraction
│  • Automatic serialize/deserialize                   │
└────────────────────┬─────────────────────────────────┘
                     │ bytes (Vec<u8>)
┌────────────────────▼─────────────────────────────────┐
│          Physical Transport                          │
│  • CanDriver (CAN-FD)                                │
│  • UsbCdcDriver (USB)                                │ ← Hardware-specific
│  • SpiDriver (SPI)                                   │
└──────────────────────────────────────────────────────┘
```

## Использование

### До (с явной сериализацией):

```rust
// Получить CAN фрейм
let frame = can.receive().await?;

// Вручную десериализовать
let msg = Message::deserialize(&frame.data)?;

// Обработать
let response = bridge.handle_message(&msg)?;

// Вручную сериализовать
let data = response.serialize()?;

// Отправить CAN фрейм
let resp_frame = CanFrame::new(node_id).with_data(&data);
can.send(resp_frame).await?;
```

### После (с transport abstraction):

```rust
// Получить сообщение (десериализация автоматическая)
let msg = transport.receive_message().await?;

// Обработать
let response = bridge.handle_message(&msg)?;

// Отправить ответ (сериализация автоматическая)
transport.send_message(&response).await?;
```

## Преимущества

### 1. **Скрытие деталей коммуникации**
- ❌ Нет прямой работы с `CanFrame`
- ❌ Нет ручной сериализации/десериализации
- ✅ Только typed `Message` структуры

### 2. **Легкая смена транспорта**
```rust
// CAN-FD
let transport = IrpcTransport::new(&mut can_driver);

// USB (в будущем)
let transport = IrpcTransport::new(&mut usb_driver);

// Код обработки одинаковый!
```

### 3. **Централизованная обработка ошибок**
```rust
pub enum TransportError {
    Serialization(ProtocolError),    // Ошибка протокола
    Deserialization(ProtocolError),  // Ошибка протокола
    CanBusError,                     // Ошибка CAN
    MessageTooLarge(usize),          // MTU exceeded
    InvalidFrame,                    // Битый фрейм
}
```

### 4. **Автоматическая фрагментация** (будущее)
```rust
// Сообщение > 64 bytes автоматически разбивается
// на несколько CAN-FD фреймов (transparent для пользователя)
transport.send_message(&large_message).await?;
```

## Реализация в can_comm task

```rust
#[embassy_executor::task]
pub async fn can_communication(node_id: u16) {
    // 1. Инициализация
    let mut can = CanDriver::new(p, node_id);
    let mut transport = IrpcTransport::new(&mut can);
    let mut bridge = JointFocBridge::new(node_id);
    
    // 2. Простой цикл обработки
    loop {
        // Receive (автоматическая десериализация)
        match transport.receive_message().await {
            Ok(Some(msg)) => {
                // Process (чистая бизнес-логика)
                if let Some(response) = bridge.handle_message(&msg) {
                    // Send (автоматическая сериализация)
                    transport.send_message(&response).await.ok();
                }
            }
            Ok(None) => {/* No message */}
            Err(e) => defmt::error!("Transport: {:?}", e),
        }
    }
}
```

## Совместимость с iRPC

### Текущее состояние:
- ✅ `IrpcTransport` реализован в firmware
- ✅ Использует `Message::serialize()` / `deserialize()`
- ✅ Полностью функционален для CAN-FD

### Будущее (когда iRPC обновится):
```rust
// iRPC будет предоставлять:
trait EmbeddedTransport {
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Option<&[u8]>>;
}

// И наш CanDriver просто имплементирует этот trait
impl EmbeddedTransport for CanDriver { ... }

// Тогда можно будет использовать:
let transport = irpc::Transport::new(can_driver);
transport.send_message(&msg).await?;
```

## Тестирование

```rust
#[cfg(test)]
mod tests {
    // Mock CAN driver для unit-тестов
    struct MockCanDriver {
        tx_buffer: Vec<CanFrame>,
        rx_buffer: Vec<CanFrame>,
    }
    
    // Тест всего цикла без реального CAN
    #[test]
    fn test_full_message_cycle() {
        let mut mock_can = MockCanDriver::new();
        let mut transport = IrpcTransport::new(&mut mock_can);
        
        // Отправка
        let msg = Message { /* ... */ };
        transport.send_message(&msg).await.unwrap();
        
        // Проверка что CAN получил правильные байты
        assert!(mock_can.tx_buffer.len() == 1);
    }
}
```

## Roadmap

### Phase 1: ✅ Completed
- [x] `IrpcTransport` для CAN-FD
- [x] Автоматическая сериализация/десериализация
- [x] Интеграция в `can_comm` task
- [x] Документация

### Phase 2: Pending iRPC update
- [ ] `EmbeddedTransport` trait в iRPC
- [ ] `impl EmbeddedTransport for CanDriver`
- [ ] Использовать `irpc::Transport` вместо своего

### Phase 3: Future
- [ ] Multi-frame поддержка (> 64 bytes)
- [ ] USB транспорт
- [ ] SPI транспорт
- [ ] Mock транспорт для тестов

## Резюме

**Firmware теперь полностью абстрагирован от CAN-FD деталей!**

Вся работа с физической коммуникацией скрыта в `IrpcTransport`,
а application layer работает только с typed `Message` структурами.

Это точно такая же концепция как в gRPC - разработчик пишет `.proto` файл,
а вся бинарная магия происходит под капотом в библиотеке.

