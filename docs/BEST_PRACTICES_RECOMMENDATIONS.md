# Firmware Best Practices - Improvement Recommendations

**Date:** 2025-11-10
**Scope:** CLN17 v2.0 Motor Controller Firmware
**Current Grade:** B+ (85/100)
**Target Grade:** A+ (95/100)

---

## Executive Summary

The CLN17 firmware has excellent architecture, good test coverage, and strong safety features. However, it's missing some **critical production-readiness** features required for motor controller deployments:

### Critical Gaps (Must Fix):
1. ❌ No hardware watchdog timer
2. ❌ No persistent storage (calibration, faults, config)
3. ⚠️ Excessive use of `.unwrap()` in initialization (25 instances)

### High Priority Improvements:
4. Comprehensive error handling with custom error types
5. Fault injection testing
6. Explicit interrupt priority configuration

---

## 🔴 Critical Priority (Must Fix for Production)

### 1. Implement Hardware Watchdog Timer

**Current Status:** ❌ Not implemented
**Risk:** System hang could cause thermal runaway or uncontrolled motor
**Impact:** CRITICAL - safety requirement

#### Why This Matters:
Motor controllers can hang due to:
- Infinite loops in control code
- Deadlocks between tasks
- Peripheral lockup (CAN, SPI, ADC)
- Memory corruption

Without a watchdog, a hung system continues to apply PWM with no monitoring = fire hazard.

#### Implementation Plan:

**File:** `src/firmware/drivers/watchdog.rs` (NEW)

```rust
/// Independent Watchdog Timer (IWDG) driver for STM32G431.
///
/// Provides hardware-level protection against system hangs.
/// If not refreshed within timeout period, forces MCU reset.

use embassy_stm32::wdg::IndependentWatchdog;

/// Watchdog configuration.
pub struct WatchdogConfig {
    /// Timeout period in milliseconds.
    /// Must be long enough for longest task execution.
    /// Recommended: 500-1000ms for motor controllers.
    pub timeout_ms: u32,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 500,  // 500ms timeout
        }
    }
}

pub struct Watchdog {
    iwdg: IndependentWatchdog<'static>,
}

impl Watchdog {
    /// Initialize and start the watchdog.
    ///
    /// **WARNING:** Once started, watchdog cannot be stopped!
    /// Must be fed regularly or MCU will reset.
    pub fn new(config: WatchdogConfig) -> Self {
        let iwdg = IndependentWatchdog::new(config.timeout_ms);

        defmt::info!("Watchdog initialized: {}ms timeout", config.timeout_ms);

        Self { iwdg }
    }

    /// Feed the watchdog (reset timeout counter).
    ///
    /// Must be called at least once per timeout period.
    #[inline]
    pub fn feed(&mut self) {
        self.iwdg.pet();
    }

    /// Check if last reset was caused by watchdog.
    pub fn was_watchdog_reset() -> bool {
        // Check RCC reset flags
        // STM32G4: RCC.CSR register bit IWDGRSTF
        // This would need to be checked before RCC reset
        // For now, return false (needs HAL support)
        false
    }
}

/// Global watchdog instance (optional, for shared access).
pub static WATCHDOG: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    Option<Watchdog>
> = embassy_sync::mutex::Mutex::new(None);
```

**File:** `src/firmware/system.rs` (MODIFY)

```rust
// In initialize() function:

pub async fn initialize(spawner: Spawner, p: Peripherals) -> ! {
    defmt::info!("=== Joint Firmware Initialization START ===");

    // Initialize watchdog FIRST (before anything can hang)
    let mut watchdog = Watchdog::new(WatchdogConfig {
        timeout_ms: 500,  // 500ms timeout
    });
    watchdog.feed();  // Initial feed

    // Check if last reset was caused by watchdog
    if Watchdog::was_watchdog_reset() {
        defmt::error!("⚠️ WATCHDOG RESET DETECTED - System recovered from hang!");
        // Could log this fault to persistent storage
    }

    // ... rest of initialization ...

    // Spawn watchdog feeder task
    spawner.spawn(watchdog_feeder(watchdog)).ok();

    // ... spawn other tasks ...
}

/// Watchdog feeder task.
///
/// Feeds watchdog every 250ms (half the timeout period for safety margin).
/// Also verifies critical tasks are still running.
#[embassy_executor::task]
async fn watchdog_feeder(mut watchdog: Watchdog) {
    use embassy_time::{Duration, Ticker};

    let mut ticker = Ticker::every(Duration::from_millis(250));
    let mut tick_count = 0u32;

    loop {
        ticker.next().await;
        tick_count += 1;

        // Feed the watchdog
        watchdog.feed();

        // Every 4 ticks (1 second), verify system health
        if tick_count % 4 == 0 {
            // Could check:
            // - FOC task is incrementing counter
            // - Power monitor is updating metrics
            // - No emergency stops active

            defmt::trace!("Watchdog fed ({}s uptime)", tick_count / 4);
        }
    }
}
```

**Testing:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_timeout_calculation() {
        // Verify timeout calculation is correct
        let config = WatchdogConfig { timeout_ms: 500 };
        // LSI frequency: 32 kHz
        // Prescaler: ...
        // Reload value: ...
        // Actual timeout should be close to 500ms
    }
}
```

**Integration Steps:**
1. Create watchdog driver (1-2 hours)
2. Add to system initialization (30 min)
3. Add feeder task (30 min)
4. Test on hardware: induce hang, verify reset (1 hour)

**Total Effort:** 3-4 hours

**Value:** CRITICAL - Fundamental safety feature

---

### 2. Implement Persistent Storage

**Current Status:** ❌ Not implemented
**Risk:** Calibration lost on power cycle, no fault history
**Impact:** CRITICAL - user experience & diagnostics

#### What Needs to Be Stored:

1. **Motor Calibration Data** (after successful calibration)
   - Inertia (J)
   - Friction coefficients
   - Torque constant (Kt)
   - Phase resistance & inductance
   - Encoder offset

2. **Configuration**
   - Motor parameters (poles, rated current, etc.)
   - Control gains (PID values)
   - Protection limits (current, voltage, temp)
   - CAN node ID

3. **Diagnostics**
   - Fault history (last 10-20 events with timestamps)
   - Total runtime counter (hours)
   - Power cycle count
   - Temperature max/min records
   - Last known good state

4. **Factory Settings**
   - Hardware version
   - Serial number
   - Manufacturing date
   - Factory calibration

#### Implementation Plan:

**File:** `src/firmware/drivers/flash_storage.rs` (NEW)

```rust
/// STM32 Internal Flash Storage for Persistent Data.
///
/// Uses last 2 pages of flash (2 KB each) for dual-bank storage:
/// - Bank A (primary): Page 62 @ 0x0801F000
/// - Bank B (backup): Page 63 @ 0x0801F800
///
/// Dual-bank provides redundancy and safe updates.

use embassy_stm32::flash::Flash;
use crc::{Crc, CRC_32_CKSUM};

/// Flash storage layout (2048 bytes per bank).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StoredData {
    /// Magic number for validity check (0xCAFEBABE).
    pub magic: u32,

    /// Data version (for migration).
    pub version: u32,

    /// Motor calibration results.
    pub calibration: CalibrationData,

    /// User configuration.
    pub config: UserConfig,

    /// Diagnostic data.
    pub diagnostics: DiagnosticData,

    /// Factory data (read-only after manufacturing).
    pub factory: FactoryData,

    /// Reserved for future use.
    pub reserved: [u8; 512],

    /// CRC32 checksum (must be last field).
    pub crc: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CalibrationData {
    pub valid: bool,
    pub inertia_kg_m2: f32,
    pub friction_static_nm: f32,
    pub friction_viscous_nm_per_rad_s: f32,
    pub torque_constant_nm_per_a: f32,
    pub phase_resistance_ohm: f32,
    pub phase_inductance_h: f32,
    pub encoder_offset_counts: i32,
    pub timestamp_s: u32,  // Unix timestamp
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserConfig {
    pub motor_poles: u8,
    pub motor_rated_current_ma: u16,
    pub max_current_ma: u16,
    pub max_voltage_mv: u32,
    pub max_temperature_c: u8,
    pub can_node_id: u8,
    pub position_kp: f32,
    pub position_ki: f32,
    pub position_kd: f32,
    pub velocity_kp: f32,
    pub velocity_ki: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiagnosticData {
    pub total_runtime_hours: u32,
    pub power_cycle_count: u32,
    pub fault_history: [FaultRecord; 10],
    pub temp_max_c: i16,
    pub temp_min_c: i16,
    pub last_fault_timestamp: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FaultRecord {
    pub fault_type: u8,  // FaultType enum as u8
    pub timestamp: u32,
    pub vbus_mv: u16,
    pub current_ma: u16,
    pub temp_c: i8,
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FactoryData {
    pub hardware_version: u16,
    pub serial_number: u32,
    pub manufacturing_date: u32,  // Unix timestamp
    pub factory_test_passed: bool,
    pub reserved: [u8; 16],
}

pub struct FlashStorage {
    flash: Flash<'static>,
}

impl FlashStorage {
    const BANK_A_ADDR: u32 = 0x0801_F000;
    const BANK_B_ADDR: u32 = 0x0801_F800;
    const MAGIC: u32 = 0xCAFE_BABE;
    const VERSION: u32 = 1;

    pub fn new(flash: Flash<'static>) -> Self {
        Self { flash }
    }

    /// Load data from flash (tries Bank A, then Bank B if corrupted).
    pub async fn load(&mut self) -> Result<StoredData, StorageError> {
        // Try Bank A first
        match self.load_bank(Self::BANK_A_ADDR).await {
            Ok(data) => {
                defmt::info!("Loaded data from Bank A");
                return Ok(data);
            }
            Err(e) => {
                defmt::warn!("Bank A invalid: {:?}, trying Bank B", e);
            }
        }

        // Try Bank B
        match self.load_bank(Self::BANK_B_ADDR).await {
            Ok(data) => {
                defmt::info!("Loaded data from Bank B");
                return Ok(data);
            }
            Err(e) => {
                defmt::error!("Bank B also invalid: {:?}", e);
                return Err(e);
            }
        }
    }

    /// Save data to both banks for redundancy.
    pub async fn save(&mut self, data: &StoredData) -> Result<(), StorageError> {
        // Calculate CRC
        let mut data_with_crc = *data;
        data_with_crc.crc = self.calculate_crc(data);

        // Erase and write Bank A
        self.erase_page(Self::BANK_A_ADDR).await?;
        self.write_bank(Self::BANK_A_ADDR, &data_with_crc).await?;

        // Verify Bank A
        let verify_a = self.load_bank(Self::BANK_A_ADDR).await?;

        // Erase and write Bank B
        self.erase_page(Self::BANK_B_ADDR).await?;
        self.write_bank(Self::BANK_B_ADDR, &data_with_crc).await?;

        defmt::info!("Saved data to both banks");
        Ok(())
    }

    /// Create default data structure.
    pub fn create_default() -> StoredData {
        StoredData {
            magic: Self::MAGIC,
            version: Self::VERSION,
            calibration: CalibrationData::default(),
            config: UserConfig::default(),
            diagnostics: DiagnosticData::default(),
            factory: FactoryData::default(),
            reserved: [0; 512],
            crc: 0,  // Will be calculated on save
        }
    }

    fn calculate_crc(&self, data: &StoredData) -> u32 {
        let crc = Crc::<u32>::new(&CRC_32_CKSUM);

        // CRC over everything except the CRC field itself
        let bytes = unsafe {
            core::slice::from_raw_parts(
                data as *const _ as *const u8,
                core::mem::size_of::<StoredData>() - 4,  // Exclude CRC field
            )
        };

        crc.checksum(bytes)
    }

    async fn load_bank(&mut self, addr: u32) -> Result<StoredData, StorageError> {
        // Read data from flash
        let mut data = StoredData::default();
        unsafe {
            let ptr = addr as *const StoredData;
            data = *ptr;
        }

        // Validate magic number
        if data.magic != Self::MAGIC {
            return Err(StorageError::InvalidMagic);
        }

        // Validate version
        if data.version != Self::VERSION {
            return Err(StorageError::VersionMismatch);
        }

        // Validate CRC
        let stored_crc = data.crc;
        let calculated_crc = self.calculate_crc(&data);
        if stored_crc != calculated_crc {
            return Err(StorageError::CrcMismatch);
        }

        Ok(data)
    }

    async fn erase_page(&mut self, addr: u32) -> Result<(), StorageError> {
        // STM32G4 flash erase by page
        self.flash.erase(addr, addr + 2048).await
            .map_err(|_| StorageError::EraseError)?;
        Ok(())
    }

    async fn write_bank(&mut self, addr: u32, data: &StoredData) -> Result<(), StorageError> {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                data as *const _ as *const u8,
                core::mem::size_of::<StoredData>(),
            )
        };

        self.flash.write(addr, bytes).await
            .map_err(|_| StorageError::WriteError)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StorageError {
    InvalidMagic,
    VersionMismatch,
    CrcMismatch,
    EraseError,
    WriteError,
}

// Default implementations
impl Default for StoredData {
    fn default() -> Self {
        Self {
            magic: FlashStorage::MAGIC,
            version: FlashStorage::VERSION,
            calibration: CalibrationData::default(),
            config: UserConfig::default(),
            diagnostics: DiagnosticData::default(),
            factory: FactoryData::default(),
            reserved: [0; 512],
            crc: 0,
        }
    }
}

impl Default for CalibrationData {
    fn default() -> Self {
        Self {
            valid: false,
            inertia_kg_m2: 0.0,
            friction_static_nm: 0.0,
            friction_viscous_nm_per_rad_s: 0.0,
            torque_constant_nm_per_a: 0.0,
            phase_resistance_ohm: 0.0,
            phase_inductance_h: 0.0,
            encoder_offset_counts: 0,
            timestamp_s: 0,
        }
    }
}

// ... other Default implementations ...
```

**Usage Example:**

```rust
// In calibration task:
pub async fn run_calibration(...) -> Result<(), CalibrationError> {
    // ... perform calibration ...

    // Save results to flash
    let mut storage = FLASH_STORAGE.lock().await;
    let mut data = storage.load().await.unwrap_or_else(|_| {
        FlashStorage::create_default()
    });

    data.calibration.valid = true;
    data.calibration.inertia_kg_m2 = results.inertia;
    data.calibration.friction_static_nm = results.friction_static;
    // ... fill in other values ...
    data.calibration.timestamp_s = get_timestamp();

    storage.save(&data).await?;

    defmt::info!("Calibration saved to flash");
    Ok(())
}

// On startup, load calibration:
let mut storage = FlashStorage::new(p.FLASH);
match storage.load().await {
    Ok(data) => {
        if data.calibration.valid {
            defmt::info!("Loaded calibration from flash");
            apply_calibration(&data.calibration);
        } else {
            defmt::warn!("No valid calibration in flash");
        }
    }
    Err(_) => {
        defmt::warn!("Flash empty or corrupted, using defaults");
    }
}
```

**Integration Steps:**
1. Create flash storage driver (4-6 hours)
2. Add CRC library to Cargo.toml (30 min)
3. Integrate with calibration system (2 hours)
4. Integrate with diagnostics (fault logging) (2 hours)
5. Test erase/write/read cycles (2 hours)
6. Test power-cycle retention (1 hour)

**Total Effort:** 11-14 hours

**Value:** CRITICAL - Essential for production deployment

---

### 3. Replace Initialization Unwraps

**Current Status:** ⚠️ 25 instances of `.unwrap()` / `.expect()` in firmware
**Risk:** Panic during initialization = bricked device
**Impact:** HIGH - robustness & user experience

#### Problem:

```rust
// Current code (system.rs):
let mut uart = Uart::new(...).expect("UART init failed");  // Line 56
spawner.spawn(uart_logger_task(uart)).ok();  // Ignores error!
spawner.spawn(can_communication(...)).ok();  // Ignores error!
```

If UART fails to initialize → panic → device unusable.

#### Solution:

**Create firmware-wide error type:**

```rust
// src/firmware/error.rs (NEW)

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum FirmwareError {
    // Initialization errors
    UartInitFailed,
    AdcInitFailed,
    PwmInitFailed,
    CanInitFailed,
    FlashInitFailed,

    // Runtime errors
    SensorReadError,
    MotorDriverFault,
    CalibrationFailed,
    CommunicationTimeout,
    StorageError,

    // Configuration errors
    InvalidParameter,
    OutOfRange,
}

pub type Result<T> = core::result::Result<T, FirmwareError>;

impl FirmwareError {
    /// Check if error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Can continue without these
            Self::UartInitFailed => true,
            Self::CanInitFailed => true,  // Can run in Step-Dir mode

            // Cannot continue without these
            Self::AdcInitFailed => false,
            Self::PwmInitFailed => false,
            Self::MotorDriverFault => false,

            _ => true,
        }
    }

    /// Get error severity.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::AdcInitFailed | Self::PwmInitFailed => ErrorSeverity::Critical,
            Self::MotorDriverFault => ErrorSeverity::Critical,
            Self::UartInitFailed | Self::CanInitFailed => ErrorSeverity::Warning,
            _ => ErrorSeverity::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorSeverity {
    Critical,  // Cannot operate
    Warning,   // Degraded mode
    Info,      // Informational only
}
```

**Update initialization code:**

```rust
// src/firmware/system.rs

pub async fn initialize(spawner: Spawner, p: Peripherals) -> ! {
    defmt::info!("=== Joint Firmware Initialization START ===");

    // Track initialization status
    let mut init_errors = heapless::Vec::<FirmwareError, 10>::new();

    // Initialize UART (non-critical - can operate without logging)
    let uart_result = Uart::new(
        p.USART3, p.PC11, p.PC10, UartIrqs,
        p.DMA1_CH1, p.DMA1_CH2,
        uart_log::uart_config(),
    );

    match uart_result {
        Ok(uart) => {
            defmt::info!("✓ UART initialized");
            spawner.spawn(uart_logger_task(uart)).ok();
        }
        Err(_) => {
            defmt::error!("✗ UART init failed - continuing without logging");
            init_errors.push(FirmwareError::UartInitFailed).ok();
        }
    }

    // Initialize ADC (CRITICAL - cannot operate without current sensing)
    let adc_result = Adc::new(p.ADC1);
    let sensors = match adc_result {
        Ok(adc) => {
            defmt::info!("✓ ADC initialized");
            Sensors::new_with_adc(adc, p.PA2, p.PA3, p.PB0, p.DMA1_CH1)
        }
        Err(_) => {
            defmt::error!("✗ ADC init FAILED - CRITICAL ERROR");
            init_errors.push(FirmwareError::AdcInitFailed).ok();
            enter_safe_mode(&init_errors).await;
        }
    };

    // Initialize PWM (CRITICAL - cannot operate without motor control)
    let pwm_result = MotorPwm::new(p.TIM2, p.PA0, p.PA1, p.PB10, p.PB11);
    let pwm = match pwm_result {
        Ok(pwm) => {
            defmt::info!("✓ PWM initialized");
            pwm
        }
        Err(_) => {
            defmt::error!("✗ PWM init FAILED - CRITICAL ERROR");
            init_errors.push(FirmwareError::PwmInitFailed).ok();
            enter_safe_mode(&init_errors).await;
        }
    };

    // ... continue with other initializations ...

    // Report initialization status
    if init_errors.is_empty() {
        defmt::info!("✓ All systems initialized successfully");
    } else {
        defmt::warn!("⚠️ Initialization completed with {} errors:", init_errors.len());
        for error in init_errors.iter() {
            defmt::warn!("  - {:?}", error);
        }
    }

    // ... spawn tasks ...
}

/// Enter safe mode when critical initialization fails.
///
/// - Blink red LED
/// - Log error via defmt
/// - Wait for watchdog reset or manual power cycle
async fn enter_safe_mode(errors: &heapless::Vec<FirmwareError, 10>) -> ! {
    defmt::error!("ENTERING SAFE MODE - Critical initialization failed!");
    defmt::error!("Errors:");
    for error in errors.iter() {
        defmt::error!("  - {:?}", error);
    }

    // Try to initialize LED for error indication
    // (even if other peripherals failed)
    if let Ok(mut led) = Output::new(p.PB13, Level::High, Speed::Low) {
        loop {
            led.toggle();
            Timer::after(Duration::from_millis(200)).await;
        }
    } else {
        // Can't even blink LED - just loop
        loop {
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}
```

**Integration Steps:**
1. Create error module (1 hour)
2. Update system.rs initialization (2-3 hours)
3. Update driver constructors to return Result (3-4 hours)
4. Add safe mode implementation (1 hour)
5. Test initialization failures (2 hours)

**Total Effort:** 9-11 hours

**Value:** HIGH - Professional error handling

---

## 🟡 High Priority (Important for Production)

### 4. Comprehensive Error Handling Throughout Firmware

**Effort:** 8-12 hours
**Value:** HIGH

Extend the error handling from initialization to runtime operations:

```rust
// Update all driver methods to return Result:

impl Sensors {
    pub async fn read_all_raw(&mut self) -> Result<[u16; 3]> {
        self.adc.read(...)
            .await
            .map_err(|_| FirmwareError::SensorReadError)
    }
}

impl MotorDriver {
    pub fn enable(&mut self) -> Result<()> {
        if self.is_fault() {
            return Err(FirmwareError::MotorDriverFault);
        }
        self.enable.set_high();
        Ok(())
    }
}
```

Use `?` operator for clean error propagation:

```rust
// In control tasks:
let readings = sensors.read_all_raw().await?;
let vbus = Sensors::raw_to_vbus_mv(readings[2]);
motor_driver.enable()?;
```

---

### 5. Fault Injection Testing

**Effort:** 4-6 hours
**Value:** HIGH

Create tests that intentionally trigger fault conditions:

```rust
#[cfg(test)]
mod fault_injection_tests {
    use super::*;

    #[embassy_executor::test]
    async fn test_overvoltage_protection() {
        let mut sensors = create_mock_sensors();
        let mut monitor = PowerMonitor::new(...);

        // Inject 55V reading (over 50V limit)
        sensors.inject_vbus(55000);

        // Run one monitoring cycle
        monitor.update(&mut sensors).await;

        // Verify emergency stop triggered
        assert!(monitor.is_emergency_stop_active());
        assert_eq!(monitor.get_fault_count(FaultType::Overvoltage), 1);
    }

    #[embassy_executor::test]
    async fn test_overcurrent_recovery() {
        // Test that system recovers after transient overcurrent
    }

    #[embassy_executor::test]
    async fn test_thermal_throttling() {
        // Inject high temperature, verify throttling kicks in
    }
}
```

---

### 6. Explicit Interrupt Priority Configuration

**Effort:** 2-3 hours
**Value:** MEDIUM-HIGH

Document and verify interrupt priorities:

```rust
// src/firmware/interrupt_config.rs (NEW)

/// Interrupt priority configuration for CLN17 v2.0.
///
/// STM32G4 has 4-bit priority (0-15, lower number = higher priority).
/// Embassy typically uses default priorities, but critical systems
/// should explicitly configure them.
///
/// Priority Scheme:
/// - 0-3:   Reserved for critical hardware (not used)
/// - 4:     FOC control loop (highest application priority)
/// - 5-6:   Time-critical tasks (Step-Dir, ADC)
/// - 7-8:   Normal priority (CAN, timers)
/// - 9-11:  Low priority (telemetry, logging)
/// - 12-15: Background tasks

pub struct InterruptPriorities {
    pub foc_timer: u8,        // TIM1 or TIM2 for FOC @ 10 kHz
    pub adc_dma: u8,           // ADC DMA for current sensing
    pub step_dir_exti: u8,     // EXTI for step pulse
    pub can_rx: u8,            // CAN receive interrupt
    pub uart_tx_dma: u8,       // UART DMA for logging
    pub systick: u8,           // SysTick for Embassy scheduler
}

impl Default for InterruptPriorities {
    fn default() -> Self {
        Self {
            foc_timer: 4,         // Highest priority
            adc_dma: 5,            // Very high
            step_dir_exti: 6,      // High
            can_rx: 7,             // Medium
            uart_tx_dma: 10,       // Low
            systick: 15,           // Lowest (only for scheduler)
        }
    }
}

pub fn configure_priorities(priorities: InterruptPriorities) {
    // Embassy typically handles this, but we can verify:

    defmt::info!("Interrupt Priorities:");
    defmt::info!("  FOC Timer:     {}", priorities.foc_timer);
    defmt::info!("  ADC DMA:       {}", priorities.adc_dma);
    defmt::info!("  Step-Dir EXTI: {}", priorities.step_dir_exti);
    defmt::info!("  CAN RX:        {}", priorities.can_rx);
    defmt::info!("  UART TX DMA:   {}", priorities.uart_tx_dma);

    // Validate no conflicts
    assert!(priorities.foc_timer < priorities.adc_dma);
    assert!(priorities.adc_dma < priorities.can_rx);
}
```

---

## 🟢 Medium Priority (Good to Have)

### 7. Expand DMA Usage

**Current:** UART TX/RX, ADC reads
**Missing:** SPI encoder reads, PWM updates
**Effort:** 4-6 hours
**Value:** MEDIUM

Reduce CPU load by using DMA for:
- SPI encoder communication (TLE5012B @ 4 MHz)
- PWM duty cycle updates (triple-buffer pattern)

---

### 8. State Timeout Detection

**Effort:** 3-4 hours
**Value:** MEDIUM

Add timeout guards to state machines:

```rust
pub struct FocController {
    state: FocState,
    state_entered_at: Instant,
    state_timeout: Duration,
}

impl FocController {
    pub fn check_state_timeout(&mut self) -> Result<()> {
        let elapsed = self.state_entered_at.elapsed();
        if elapsed > self.state_timeout {
            defmt::error!("State timeout in {:?} after {:?}",
                         self.state, elapsed);
            self.enter_fault_state();
            return Err(FirmwareError::StateTimeout);
        }
        Ok(())
    }
}
```

---

### 9. Architecture Documentation

**Effort:** 4-6 hours
**Value:** MEDIUM

Create visual documentation:
- System block diagram
- Task interaction diagram
- Data flow diagram
- State machine diagrams
- Memory map

---

## 🔵 Low Priority (Nice to Have)

### 10. Test Coverage Metrics

Use `cargo-tarpaulin` or `grcov` to measure coverage. Target: >80% for safety-critical code.

### 11. Hardware-in-Loop (HIL) Testing

Set up automated testing with real hardware for continuous validation.

### 12. MISRA-C / Safety Coding Standard

Consider following safety standards if targeting safety-critical applications (e.g., ISO 13849 for machinery).

---

## Implementation Roadmap

### Week 1: Critical Fixes
- Day 1-2: Watchdog timer (3-4 hours)
- Day 3-5: Persistent storage (11-14 hours)

### Week 2: Error Handling
- Day 1-3: Replace unwraps (9-11 hours)
- Day 4-5: Runtime error handling (8-12 hours)

### Week 3: Testing & Documentation
- Day 1-2: Fault injection tests (4-6 hours)
- Day 3: Interrupt priorities (2-3 hours)
- Day 4-5: Documentation (4-6 hours)

**Total Effort:** ~50-70 hours (1.5-2 weeks full-time)

---

## Expected Outcomes

### Before Improvements:
```
Grade: B+ (85/100)
- ✅ Strong architecture
- ✅ Good test coverage
- ⚠️ Missing watchdog
- ⚠️ No persistent storage
- ⚠️ Panic-prone initialization
```

### After Critical Fixes:
```
Grade: A- (90/100)
- ✅ Hardware watchdog active
- ✅ Calibration persistence
- ✅ Robust error handling
- ✅ Safe mode fallback
- ✅ Production-ready
```

### After All Improvements:
```
Grade: A+ (95/100)
- ✅ All critical features
- ✅ Comprehensive error handling
- ✅ Extensive testing
- ✅ Full documentation
- ✅ Industry-grade motor controller
```

---

## Priority Summary

| Feature | Priority | Effort | Value | Status |
|---------|----------|--------|-------|--------|
| Watchdog timer | 🔴 Critical | 3-4h | Critical | ❌ Not started |
| Persistent storage | 🔴 Critical | 11-14h | Critical | ❌ Not started |
| Init error handling | 🔴 Critical | 9-11h | High | ❌ Not started |
| Runtime error handling | 🟡 High | 8-12h | High | ⚠️ Partial |
| Fault injection tests | 🟡 High | 4-6h | High | ❌ Not started |
| Interrupt priorities | 🟡 High | 2-3h | Medium | ⚠️ Default only |
| DMA expansion | 🟢 Medium | 4-6h | Medium | ⚠️ Partial |
| State timeouts | 🟢 Medium | 3-4h | Medium | ❌ Not started |
| Architecture docs | 🟢 Medium | 4-6h | Medium | ⚠️ Partial |
| Test coverage | 🔵 Low | 2-3h | Low | ⚠️ Unknown |

---

**Next Recommended Action:** Implement watchdog timer (highest ROI: 3 hours for critical safety feature).
