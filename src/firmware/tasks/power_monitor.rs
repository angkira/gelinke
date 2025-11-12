/// Power Monitoring and Protection Task
///
/// Provides comprehensive power management for CLN17 V2.0:
/// - Continuous voltage/current monitoring @ 100 Hz
/// - Overcurrent protection (RMS + peak)
/// - Overvoltage/undervoltage protection
/// - MCU thermal management with throttling
/// - Automatic fault recovery
/// - Power metrics tracking

use embassy_time::{Duration, Ticker, Timer};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::firmware::drivers::adc::{
    Sensors, RmsCalculator, current_limits::*,
    TEMP_SHUTDOWN_C, TEMP_THROTTLE_START_C,
};
use crate::firmware::drivers::motor_driver::MotorDriver;
use crate::firmware::drivers::status_leds::{StatusLeds, LedColor};
use crate::firmware::tasks::thermal_throttle;

/// Power metrics shared across tasks.
#[derive(Clone, Copy)]
pub struct PowerMetrics {
    /// Supply voltage (millivolts)
    pub vbus_mv: u32,
    /// Phase A current (milliamps, signed)
    pub ia_ma: i32,
    /// Phase B current (milliamps, signed)
    pub ib_ma: i32,
    /// RMS current (milliamps)
    pub i_rms_ma: f32,
    /// Instantaneous power (milliwatts)
    pub power_mw: u32,
    /// MCU temperature (degrees C)
    pub mcu_temp_c: f32,
    /// Thermal throttle factor (0.0 to 1.0)
    pub throttle_factor: f32,
    /// Accumulated energy (milliwatt-hours)
    pub energy_mwh: u32,
    /// Accumulated charge (milliamp-hours)
    pub charge_mah: u32,
    /// Active time (milliseconds)
    pub active_time_ms: u32,
    /// Fault counters
    pub faults: FaultCounters,
}

impl PowerMetrics {
    pub const fn new() -> Self {
        Self {
            vbus_mv: 0,
            ia_ma: 0,
            ib_ma: 0,
            i_rms_ma: 0.0,
            power_mw: 0,
            mcu_temp_c: 25.0,
            throttle_factor: 1.0,
            energy_mwh: 0,
            charge_mah: 0,
            active_time_ms: 0,
            faults: FaultCounters::new(),
        }
    }
}

/// Fault event counters.
#[derive(Clone, Copy)]
pub struct FaultCounters {
    pub overcurrent_events: u16,
    pub overvoltage_events: u16,
    pub undervoltage_events: u16,
    pub overtemp_events: u16,
    pub driver_fault_events: u16,
    pub emergency_stops: u16,
}

impl FaultCounters {
    pub const fn new() -> Self {
        Self {
            overcurrent_events: 0,
            overvoltage_events: 0,
            undervoltage_events: 0,
            overtemp_events: 0,
            driver_fault_events: 0,
            emergency_stops: 0,
        }
    }

    pub fn total(&self) -> u16 {
        self.overcurrent_events
            .saturating_add(self.overvoltage_events)
            .saturating_add(self.undervoltage_events)
            .saturating_add(self.overtemp_events)
            .saturating_add(self.driver_fault_events)
    }
}

/// Shared power metrics state (thread-safe).
pub static POWER_METRICS: Mutex<CriticalSectionRawMutex, PowerMetrics> =
    Mutex::new(PowerMetrics::new());

/// Current sensor calibration offsets.
///
/// These should be calibrated at startup with motor disabled.
static CURRENT_OFFSETS: Mutex<CriticalSectionRawMutex, [u16; 2]> =
    Mutex::new([2048, 2048]);  // Default to mid-scale

/// Power monitoring task @ 100 Hz.
///
/// This task runs continuously to monitor and protect the power system.
/// It cannot be stopped once started.
///
/// # Safety Features
/// - Overvoltage protection (>50V)
/// - Undervoltage protection (<8V)
/// - Overcurrent protection (peak >2.5A, RMS >1.75A)
/// - Thermal throttling (70°C start, 85°C shutdown)
/// - Automatic fault recovery (3 attempts)
/// - Voltage sag detection (brownout prediction)
#[embassy_executor::task]
pub async fn power_monitor(
    mut sensors: Sensors<'static>,
    mut motor_driver: MotorDriver,
    mut status_leds: StatusLeds,
) {
    defmt::info!("Power monitor task starting");

    // Calibrate current sensors at startup
    defmt::info!("Calibrating current sensors...");
    let offsets = sensors.calibrate_current_offsets(100).await;
    {
        let mut offset_lock = CURRENT_OFFSETS.lock().await;
        *offset_lock = offsets;
    }
    defmt::info!("Current offsets: A={}, B={}", offsets[0], offsets[1]);

    // Initialize RMS calculator
    let mut rms_calc = RmsCalculator::new();

    // Monitoring loop @ 100 Hz
    let mut ticker = Ticker::every(Duration::from_millis(10));
    let mut tick_counter: u32 = 0;

    // Voltage sag detection
    let mut vbus_samples = [0u32; 10];
    let mut vbus_sample_idx = 0;

    // Fault recovery state
    let mut fault_recovery_attempts = 0;
    const MAX_RECOVERY_ATTEMPTS: u8 = 3;

    // Temperature read counter (every 1 second = 100 ticks)
    let mut temp_read_counter = 0;
    let mut mcu_temp = 25.0f32;

    loop {
        ticker.next().await;
        tick_counter = tick_counter.wrapping_add(1);

        // === READ SENSORS ===
        let [ia_raw, ib_raw, vbus_raw] = sensors.read_all_raw().await;

        let offsets = CURRENT_OFFSETS.lock().await;
        let ia_ma = Sensors::raw_to_milliamps(ia_raw, offsets[0]);
        let ib_ma = Sensors::raw_to_milliamps(ib_raw, offsets[1]);
        drop(offsets);

        let vbus_mv = Sensors::raw_to_vbus_mv(vbus_raw);

        // Update RMS calculator
        let i_rms = rms_calc.update(ia_ma, ib_ma);

        // === MCU TEMPERATURE (every 1 second) ===
        temp_read_counter += 1;
        if temp_read_counter >= 100 {
            temp_read_counter = 0;
            // Note: This may fail if embassy doesn't support internal temp sensor yet
            // In that case, we'll use a default value
            #[cfg(not(feature = "renode-mock"))]
            {
                mcu_temp = sensors.read_mcu_temperature().await;
            }
            #[cfg(feature = "renode-mock")]
            {
                // Mock temperature for Renode
                mcu_temp = 35.0;
            }
        }

        // Calculate thermal throttle factor
        let throttle = Sensors::get_thermal_throttle(mcu_temp);

        // Update lockless throttle state for FOC/Step-Dir tasks (10 kHz loops)
        thermal_throttle::set_throttle_factor(throttle);

        // === CRITICAL PROTECTION CHECKS ===

        let mut fault_detected = false;
        let mut emergency = false;

        // 1. OVERVOLTAGE PROTECTION (>50V)
        if Sensors::is_vbus_overvoltage(vbus_mv) {
            defmt::error!("OVERVOLTAGE: {} mV (limit: 50000 mV)", vbus_mv);
            motor_driver.emergency_stop();
            status_leds.set_color(LedColor::Red);

            let mut metrics = POWER_METRICS.lock().await;
            metrics.faults.overvoltage_events = metrics.faults.overvoltage_events.saturating_add(1);
            metrics.faults.emergency_stops = metrics.faults.emergency_stops.saturating_add(1);
            drop(metrics);

            emergency = true;
            fault_detected = true;
        }

        // 2. UNDERVOLTAGE PROTECTION (<8V)
        if Sensors::is_vbus_undervoltage(vbus_mv) {
            defmt::error!("UNDERVOLTAGE: {} mV (limit: 8000 mV)", vbus_mv);
            motor_driver.emergency_stop();
            status_leds.set_color(LedColor::Red);

            let mut metrics = POWER_METRICS.lock().await;
            metrics.faults.undervoltage_events = metrics.faults.undervoltage_events.saturating_add(1);
            metrics.faults.emergency_stops = metrics.faults.emergency_stops.saturating_add(1);
            drop(metrics);

            emergency = true;
            fault_detected = true;
        }

        // 3. PEAK OVERCURRENT PROTECTION (>2.5A)
        let i_peak = ia_ma.abs() + ib_ma.abs();
        if i_peak > MAX_PEAK_CURRENT_MA {
            defmt::error!("PEAK OVERCURRENT: {} mA (limit: {} mA)", i_peak, MAX_PEAK_CURRENT_MA);
            motor_driver.emergency_stop();
            status_leds.set_color(LedColor::Red);

            let mut metrics = POWER_METRICS.lock().await;
            metrics.faults.overcurrent_events = metrics.faults.overcurrent_events.saturating_add(1);
            metrics.faults.emergency_stops = metrics.faults.emergency_stops.saturating_add(1);
            drop(metrics);

            emergency = true;
            fault_detected = true;
        }

        // 4. RMS OVERCURRENT PROTECTION (>1.75A)
        // Only check after warmup period
        if rms_calc.is_warmed_up() && i_rms > MAX_RMS_CURRENT_MA {
            defmt::warn!("RMS OVERCURRENT: {} mA (limit: {} mA)", i_rms as u32, MAX_RMS_CURRENT_MA);

            // Gradual current limiting (not emergency stop)
            // This would be implemented by reducing PWM duty cycle
            // For now, just log the warning

            let mut metrics = POWER_METRICS.lock().await;
            metrics.faults.overcurrent_events = metrics.faults.overcurrent_events.saturating_add(1);
            drop(metrics);

            status_leds.set_color(LedColor::Yellow);
        }

        // 5. MCU OVERTEMPERATURE PROTECTION
        if !Sensors::is_mcu_temp_safe(mcu_temp) {
            defmt::error!("MCU OVERTEMP: {}°C (limit: {}°C)", mcu_temp, TEMP_SHUTDOWN_C);
            motor_driver.emergency_stop();
            status_leds.set_color(LedColor::Red);

            let mut metrics = POWER_METRICS.lock().await;
            metrics.faults.overtemp_events = metrics.faults.overtemp_events.saturating_add(1);
            metrics.faults.emergency_stops = metrics.faults.emergency_stops.saturating_add(1);
            drop(metrics);

            emergency = true;
            fault_detected = true;
        } else if Sensors::is_thermal_throttle_active(mcu_temp) {
            // Thermal throttling active
            if tick_counter % 100 == 0 {  // Log every 1 second
                defmt::warn!("Thermal throttle: {}% @ {}°C",
                           (throttle * 100.0) as u32, mcu_temp);
            }
            status_leds.set_color(LedColor::Yellow);
        }

        // 6. DRV8844 FAULT DETECTION
        if motor_driver.is_fault() {
            defmt::error!("DRV8844 fault detected");
            motor_driver.disable();
            status_leds.set_color(LedColor::Red);

            let mut metrics = POWER_METRICS.lock().await;
            metrics.faults.driver_fault_events = metrics.faults.driver_fault_events.saturating_add(1);
            drop(metrics);

            fault_detected = true;

            // Attempt automatic recovery (if not emergency)
            if !emergency && fault_recovery_attempts < MAX_RECOVERY_ATTEMPTS {
                fault_recovery_attempts += 1;
                defmt::warn!("Attempting fault recovery ({}/{})",
                           fault_recovery_attempts, MAX_RECOVERY_ATTEMPTS);

                Timer::after(Duration::from_millis(100)).await;
                motor_driver.reset();
                Timer::after(Duration::from_millis(50)).await;

                // Check if fault cleared
                if !motor_driver.is_fault() {
                    defmt::info!("Fault recovered successfully");
                    fault_recovery_attempts = 0;
                    status_leds.set_color(LedColor::Green);
                }
            }
        } else if fault_recovery_attempts > 0 && tick_counter % 1000 == 0 {
            // Reset recovery counter after 10 seconds of no faults
            fault_recovery_attempts = 0;
        }

        // === VOLTAGE SAG DETECTION (Brownout Prediction) ===
        vbus_samples[vbus_sample_idx] = vbus_mv;
        vbus_sample_idx = (vbus_sample_idx + 1) % 10;

        if tick_counter % 10 == 0 {  // Every 100ms
            let vbus_avg: u32 = vbus_samples.iter().sum::<u32>() / 10;
            if vbus_avg < 10000 && motor_driver.is_enabled() {
                defmt::warn!("Voltage sag: {} mV average (brownout risk)", vbus_avg);
                // Could implement preemptive current reduction here
            }
        }

        // === UPDATE POWER METRICS ===
        let i_total_ma = (ia_ma.abs() + ib_ma.abs()) as u32;
        let power_mw = (vbus_mv * i_total_ma) / 1000;

        // Energy accumulation (E += P * dt)
        // dt = 10ms = 10/3600000 hours
        let energy_increment = (power_mw * 10) / 3600000;  // mWh
        let charge_increment = (i_total_ma * 10) / 3600000;  // mAh

        {
            let mut metrics = POWER_METRICS.lock().await;
            metrics.vbus_mv = vbus_mv;
            metrics.ia_ma = ia_ma;
            metrics.ib_ma = ib_ma;
            metrics.i_rms_ma = i_rms;
            metrics.power_mw = power_mw;
            metrics.mcu_temp_c = mcu_temp;
            metrics.throttle_factor = throttle;
            metrics.energy_mwh = metrics.energy_mwh.saturating_add(energy_increment);
            metrics.charge_mah = metrics.charge_mah.saturating_add(charge_increment);

            if motor_driver.is_enabled() {
                metrics.active_time_ms = metrics.active_time_ms.saturating_add(10);
            }
        }

        // === STATUS LED UPDATE ===
        if !fault_detected {
            if motor_driver.is_enabled() {
                if throttle < 1.0 {
                    status_leds.set_color(LedColor::Yellow);  // Throttling
                } else {
                    status_leds.set_color(LedColor::Green);  // Normal operation
                }
            } else {
                status_leds.set_color(LedColor::Blue);  // Idle
            }
        }

        // === PERIODIC LOGGING (every 10 seconds) ===
        if tick_counter % 1000 == 0 {
            let metrics = POWER_METRICS.lock().await;
            defmt::info!("Power: V={} mV, I_RMS={} mA, P={} mW, T={}°C, Throttle={}%",
                       metrics.vbus_mv,
                       metrics.i_rms_ma as u32,
                       metrics.power_mw,
                       metrics.mcu_temp_c,
                       (metrics.throttle_factor * 100.0) as u32);
        }
    }
}
