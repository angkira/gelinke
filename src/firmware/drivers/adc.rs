use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::peripherals::ADC1;

/// ADC configuration for current sensing and voltage monitoring.
pub const ADC_SAMPLE_TIME: SampleTime = SampleTime::CYCLES12_5;

/// DRV8844 current sense output characteristics.
/// The DRV8844 outputs an analog voltage proportional to motor current.
/// Typical: 0.2V/A at AISEN/BISEN pins.
pub const DRV8844_CURRENT_SENSE_V_PER_A: f32 = 0.2;

/// ADC reference voltage in millivolts.
pub const VREF_MV: u32 = 3300;

/// Supply voltage divider ratio (Vbus to ADC).
/// CLN17 V2.0 uses voltage divider to scale 8-48V down to 0-3.3V.
/// Typical ratio: 1:15 (R1=14k, R2=1k)
pub const VBUS_DIVIDER_RATIO: f32 = 15.0;

/// MCU internal temperature sensor calibration values (STM32G431CB).
/// These values are from the STM32G4 datasheet.
/// V_SENSE = voltage at 25°C (typically 760 mV)
/// AVG_SLOPE = temperature coefficient (typically 2.5 mV/°C)
pub const TEMP_V25_MV: f32 = 760.0;
pub const TEMP_AVG_SLOPE_MV_PER_C: f32 = 2.5;

/// Thermal management thresholds.
pub const TEMP_THROTTLE_START_C: f32 = 70.0;  // Begin current reduction
pub const TEMP_THROTTLE_HEAVY_C: f32 = 80.0;  // Heavy current reduction
pub const TEMP_SHUTDOWN_C: f32 = 85.0;         // Emergency shutdown

/// Current and voltage sensor using ADC1 with DMA.
///
/// CLN17 V2.0 hardware connections:
/// - PA3 (ADC1_IN4): DRV8844 AISEN (Phase A current)
/// - PB0 (ADC1_IN15): DRV8844 BISEN (Phase B current)
/// - PA2 (ADC1_IN3): Vbus voltage divider
/// - DMA1_CH3: ADC1 DMA transfers (via DMAMUX)
pub struct Sensors {
    adc: Adc<'static, ADC1>,
    dma: embassy_stm32::Peri<'static, embassy_stm32::peripherals::DMA1_CH3>,
    current_a: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    current_b: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    vbus: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    buffer: [u16; 3],
}

impl Sensors {
    /// Create a new sensor instance.
    ///
    /// # Arguments
    /// * `adc1` - ADC1 peripheral
    /// * `dma1_ch3` - DMA1 Channel 3 for ADC transfers (via DMAMUX)
    /// * `pa3` - PA3 pin for Phase A current sensing (ADC1_IN4)
    /// * `pb0` - PB0 pin for Phase B current sensing (ADC1_IN15)
    /// * `pa2` - PA2 pin for Vbus voltage monitoring (ADC1_IN3)
    pub fn new(
        adc1: embassy_stm32::peripherals::ADC1,
        dma1_ch3: embassy_stm32::peripherals::DMA1_CH3,
        pa3: embassy_stm32::peripherals::PA3,
        pb0: embassy_stm32::peripherals::PB0,
        pa2: embassy_stm32::peripherals::PA2,
    ) -> Self {
        let adc = Adc::new(adc1);

        // Current sensing from DRV8844
        let current_a = pa3.degrade_adc();  // ADC1_IN4
        let current_b = pb0.degrade_adc();  // ADC1_IN15

        // Supply voltage monitoring
        let vbus = pa2.degrade_adc();  // ADC1_IN3

        let dma = dma1_ch3;

        Self {
            adc,
            dma,
            current_a,
            current_b,
            vbus,
            buffer: [0; 3],
        }
    }

    /// Read raw ADC values from all sensors.
    ///
    /// Returns [current_a_raw, current_b_raw, vbus_raw]
    pub async fn read_all_raw(&mut self) -> [u16; 3] {
        self.adc
            .read(
                self.dma.reborrow(),
                [
                    (&mut self.current_a, ADC_SAMPLE_TIME),
                    (&mut self.current_b, ADC_SAMPLE_TIME),
                    (&mut self.vbus, ADC_SAMPLE_TIME),
                ]
                .into_iter(),
                &mut self.buffer,
            )
            .await;

        self.buffer
    }

    /// Read raw ADC values from current sensors only.
    ///
    /// Returns [phase_a_raw, phase_b_raw]
    pub async fn read_currents_raw(&mut self) -> [u16; 2] {
        let mut buffer = [0u16; 2];
        self.adc
            .read(
                self.dma.reborrow(),
                [
                    (&mut self.current_a, ADC_SAMPLE_TIME),
                    (&mut self.current_b, ADC_SAMPLE_TIME),
                ]
                .into_iter(),
                &mut buffer,
            )
            .await;

        buffer
    }

    /// Read raw Vbus ADC value.
    pub async fn read_vbus_raw(&mut self) -> u16 {
        let mut buffer = [0u16; 1];
        self.adc
            .read(
                self.dma.reborrow(),
                [(&mut self.vbus, ADC_SAMPLE_TIME)].into_iter(),
                &mut buffer,
            )
            .await;

        buffer[0]
    }

    /// Convert raw ADC value to current in milliamps (DRV8844 current sense).
    ///
    /// The DRV8844 outputs ~0.2V per Amp of motor current.
    ///
    /// # Arguments
    /// * `raw` - Raw ADC value (0-4095 for 12-bit ADC)
    /// * `offset` - Calibrated zero-current ADC offset
    pub fn raw_to_milliamps(raw: u16, offset: u16) -> i32 {
        // Convert ADC counts to voltage
        let voltage_mv = (raw as i32 * VREF_MV as i32) / 4096;
        let offset_mv = (offset as i32 * VREF_MV as i32) / 4096;
        let diff_mv = voltage_mv - offset_mv;

        // Convert voltage to current using DRV8844 transfer function
        // I(mA) = V(mV) / (V_per_A * 1000)
        let current_ma = (diff_mv as f32 / (DRV8844_CURRENT_SENSE_V_PER_A * 1000.0)) as i32;

        current_ma
    }

    /// Convert raw ADC value to supply voltage in millivolts.
    ///
    /// # Arguments
    /// * `raw` - Raw ADC value from Vbus divider
    pub fn raw_to_vbus_mv(raw: u16) -> u32 {
        // ADC voltage
        let adc_mv = (raw as u32 * VREF_MV) / 4096;

        // Scale by divider ratio
        (adc_mv as f32 * VBUS_DIVIDER_RATIO) as u32
    }

    /// Calibrate current sensor offsets by averaging multiple samples at zero current.
    ///
    /// **IMPORTANT:** Motor must be disabled (no current flowing) during calibration.
    ///
    /// Returns [phase_a_offset, phase_b_offset]
    pub async fn calibrate_current_offsets(&mut self, samples: usize) -> [u16; 2] {
        let mut sum_a: u32 = 0;
        let mut sum_b: u32 = 0;

        for _ in 0..samples {
            let [a, b] = self.read_currents_raw().await;
            sum_a += a as u32;
            sum_b += b as u32;
        }

        [
            (sum_a / samples as u32) as u16,
            (sum_b / samples as u32) as u16,
        ]
    }

    /// Check if supply voltage is within safe operating range.
    ///
    /// CLN17 V2.0 spec: 8-48V nominal
    pub fn is_vbus_in_range(vbus_mv: u32) -> bool {
        vbus_mv >= 8000 && vbus_mv <= 48000
    }

    /// Check if supply voltage is critically low.
    pub fn is_vbus_undervoltage(vbus_mv: u32) -> bool {
        vbus_mv < 8000
    }

    /// Check if supply voltage is critically high.
    pub fn is_vbus_overvoltage(vbus_mv: u32) -> bool {
        vbus_mv > 50000  // 50V absolute max
    }

    /// Read MCU internal temperature sensor.
    ///
    /// Returns temperature in degrees Celsius.
    ///
    /// Uses the STM32G4 internal temperature sensor formula:
    /// T(°C) = (V_SENSE - V_25°C) / Avg_Slope + 25
    ///
    /// **Note:** This requires enabling the temperature sensor channel in ADC.
    /// The temperature sensor is connected to ADC internal channel.
    pub async fn read_mcu_temperature(&mut self) -> f32 {
        // Read internal temperature sensor
        // Note: Embassy may need specific API for internal channels
        // This is a simplified version - actual implementation may vary
        let temp_raw = self.adc.read_internal(&mut embassy_stm32::adc::Temperature).await;

        // Convert to millivolts
        let temp_mv = (temp_raw as u32 * VREF_MV) / 4096;

        // Calculate temperature using calibration formula
        let temp_c = ((temp_mv as f32 - TEMP_V25_MV) / TEMP_AVG_SLOPE_MV_PER_C) + 25.0;

        temp_c
    }

    /// Check if MCU temperature is within safe operating range.
    pub fn is_mcu_temp_safe(temp_c: f32) -> bool {
        temp_c < TEMP_SHUTDOWN_C
    }

    /// Get thermal throttle factor based on MCU temperature.
    ///
    /// Returns a multiplier (0.0 to 1.0) to apply to current limits:
    /// - 1.0: Full power (temp < 70°C)
    /// - 0.7: Reduced power (temp 70-80°C)
    /// - 0.5: Heavy reduction (temp 80-85°C)
    /// - 0.0: Emergency shutdown (temp > 85°C)
    pub fn get_thermal_throttle(temp_c: f32) -> f32 {
        if temp_c < TEMP_THROTTLE_START_C {
            1.0  // Full power
        } else if temp_c < TEMP_THROTTLE_HEAVY_C {
            // Linear interpolation between 1.0 and 0.7
            let ratio = (temp_c - TEMP_THROTTLE_START_C) / (TEMP_THROTTLE_HEAVY_C - TEMP_THROTTLE_START_C);
            1.0 - (ratio * 0.3)
        } else if temp_c < TEMP_SHUTDOWN_C {
            // Linear interpolation between 0.7 and 0.5
            let ratio = (temp_c - TEMP_THROTTLE_HEAVY_C) / (TEMP_SHUTDOWN_C - TEMP_THROTTLE_HEAVY_C);
            0.7 - (ratio * 0.2)
        } else {
            0.0  // Emergency shutdown
        }
    }

    /// Check if thermal throttling is active.
    pub fn is_thermal_throttle_active(temp_c: f32) -> bool {
        temp_c >= TEMP_THROTTLE_START_C
    }
}

// Legacy compatibility aliases
pub type CurrentSensors = Sensors;

/// RMS current calculator for motor protection.
///
/// Calculates root-mean-square (RMS) current over a sliding window
/// to protect against exceeding DRV8844 thermal limits (1.75A RMS).
///
/// Uses I²t calculation: I_RMS = sqrt(mean(I_A² + I_B²) / 2)
pub struct RmsCalculator {
    /// Buffer of I² samples (milliamps squared)
    i_sq_buffer: [f32; 100],
    /// Current write index in circular buffer
    index: usize,
    /// Number of samples accumulated (up to buffer size)
    count: usize,
}

impl RmsCalculator {
    /// Create a new RMS calculator.
    ///
    /// # Parameters
    /// - Window size: 100 samples
    /// - At 10 kHz sampling: 10ms window
    /// - At 1 kHz sampling: 100ms window
    pub const fn new() -> Self {
        Self {
            i_sq_buffer: [0.0; 100],
            index: 0,
            count: 0,
        }
    }

    /// Update with new current samples and return current RMS value.
    ///
    /// # Arguments
    /// * `ia_ma` - Phase A current in milliamps (signed)
    /// * `ib_ma` - Phase B current in milliamps (signed)
    ///
    /// # Returns
    /// RMS current in milliamps
    pub fn update(&mut self, ia_ma: i32, ib_ma: i32) -> f32 {
        // Calculate I² for both phases
        let i_sq_a = (ia_ma as f32).powi(2);
        let i_sq_b = (ib_ma as f32).powi(2);

        // Combined I² (average of both phases)
        let i_sq_combined = (i_sq_a + i_sq_b) / 2.0;

        // Store in circular buffer
        self.i_sq_buffer[self.index] = i_sq_combined;
        self.index = (self.index + 1) % 100;

        // Track number of samples (saturate at buffer size)
        if self.count < 100 {
            self.count += 1;
        }

        // Calculate mean I²
        let sum: f32 = self.i_sq_buffer.iter().take(self.count).sum();
        let mean_i_sq = sum / (self.count as f32);

        // Return RMS: sqrt(mean(I²))
        mean_i_sq.sqrt()
    }

    /// Get current RMS value without updating.
    pub fn get_rms(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }

        let sum: f32 = self.i_sq_buffer.iter().take(self.count).sum();
        let mean_i_sq = sum / (self.count as f32);
        mean_i_sq.sqrt()
    }

    /// Reset the calculator.
    pub fn reset(&mut self) {
        self.i_sq_buffer = [0.0; 100];
        self.index = 0;
        self.count = 0;
    }

    /// Check if buffer is full (warm-up complete).
    pub fn is_warmed_up(&self) -> bool {
        self.count >= 100
    }
}

/// Current limit constants for DRV8844.
pub mod current_limits {
    /// DRV8844 maximum RMS current (continuous).
    pub const MAX_RMS_CURRENT_MA: f32 = 1750.0;  // 1.75A RMS

    /// Peak current limit (transient).
    /// DRV8844 can handle brief peaks up to 2.5A.
    pub const MAX_PEAK_CURRENT_MA: i32 = 2500;

    /// Software current limit with safety margin.
    /// Set to 90% of hardware limit for protection.
    pub const SOFTWARE_CURRENT_LIMIT_MA: f32 = 1575.0;  // 90% of 1.75A

    /// Emergency overcurrent threshold.
    /// Immediate shutdown if exceeded.
    pub const EMERGENCY_CURRENT_MA: i32 = 3000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_conversion_zero() {
        // At zero current, raw should equal offset
        let offset = 2048;
        let current = Sensors::raw_to_milliamps(2048, offset);
        assert_eq!(current, 0);
    }

    #[test]
    fn current_conversion_positive() {
        // Test positive current conversion
        // DRV8844: 0.2V/A, so 1A = 0.2V = 248 ADC counts @ 3.3V ref
        let offset = 2048;
        let raw = 2048 + 248;  // +1A
        let current = Sensors::raw_to_milliamps(raw, offset);
        // Should be approximately 1000 mA
        assert!(current > 900 && current < 1100);
    }

    #[test]
    fn current_conversion_negative() {
        // Test negative current conversion
        let offset = 2048;
        let raw = 2048 - 248;  // -1A
        let current = Sensors::raw_to_milliamps(raw, offset);
        // Should be approximately -1000 mA
        assert!(current < -900 && current > -1100);
    }

    #[test]
    fn vbus_conversion() {
        // Test Vbus voltage conversion
        // Example: 24V supply
        // After 1:15 divider: 24V / 15 = 1.6V
        // ADC: 1600mV / 3300mV * 4096 = 1984 counts
        let raw = 1984;
        let vbus = Sensors::raw_to_vbus_mv(raw);
        // Should be approximately 24000 mV
        assert!(vbus > 23000 && vbus < 25000);
    }

    #[test]
    fn vbus_range_check() {
        assert!(Sensors::is_vbus_in_range(12000));   // 12V OK
        assert!(Sensors::is_vbus_in_range(24000));   // 24V OK
        assert!(Sensors::is_vbus_in_range(48000));   // 48V OK
        assert!(!Sensors::is_vbus_in_range(5000));   // 5V too low
        assert!(!Sensors::is_vbus_in_range(55000));  // 55V too high
    }

    #[test]
    fn vbus_undervoltage() {
        assert!(Sensors::is_vbus_undervoltage(7000));
        assert!(!Sensors::is_vbus_undervoltage(12000));
    }

    #[test]
    fn vbus_overvoltage() {
        assert!(Sensors::is_vbus_overvoltage(55000));
        assert!(!Sensors::is_vbus_overvoltage(24000));
    }

    #[test]
    fn thermal_throttle_full_power() {
        // Below threshold - full power
        assert_eq!(Sensors::get_thermal_throttle(25.0), 1.0);
        assert_eq!(Sensors::get_thermal_throttle(69.9), 1.0);
    }

    #[test]
    fn thermal_throttle_reduced() {
        // At 70°C threshold - should start reducing
        let throttle = Sensors::get_thermal_throttle(70.0);
        assert!(throttle >= 0.99 && throttle <= 1.0);

        // At 75°C (midpoint) - approximately 0.85
        let throttle = Sensors::get_thermal_throttle(75.0);
        assert!(throttle > 0.8 && throttle < 0.9);

        // At 80°C - approximately 0.7
        let throttle = Sensors::get_thermal_throttle(80.0);
        assert!(throttle >= 0.69 && throttle <= 0.71);
    }

    #[test]
    fn thermal_throttle_heavy() {
        // Between 80-85°C
        let throttle = Sensors::get_thermal_throttle(82.5);
        assert!(throttle > 0.5 && throttle < 0.7);
    }

    #[test]
    fn thermal_throttle_shutdown() {
        // At or above 85°C
        assert_eq!(Sensors::get_thermal_throttle(85.0), 0.0);
        assert_eq!(Sensors::get_thermal_throttle(90.0), 0.0);
    }

    #[test]
    fn mcu_temp_safe() {
        assert!(Sensors::is_mcu_temp_safe(25.0));
        assert!(Sensors::is_mcu_temp_safe(84.9));
        assert!(!Sensors::is_mcu_temp_safe(85.0));
        assert!(!Sensors::is_mcu_temp_safe(100.0));
    }

    #[test]
    fn rms_calculator_zero_current() {
        let mut calc = RmsCalculator::new();
        let rms = calc.update(0, 0);
        assert_eq!(rms, 0.0);
    }

    #[test]
    fn rms_calculator_constant_current() {
        let mut calc = RmsCalculator::new();

        // Feed constant 1000mA on both phases
        for _ in 0..100 {
            calc.update(1000, 1000);
        }

        let rms = calc.get_rms();
        // RMS of constant 1000mA should be approximately 1000mA
        // sqrt(mean((1000² + 1000²)/2)) = sqrt(1000000) = 1000
        assert!(rms > 990.0 && rms < 1010.0);
    }

    #[test]
    fn rms_calculator_single_phase() {
        let mut calc = RmsCalculator::new();

        // 1000mA on phase A, 0 on phase B
        for _ in 0..100 {
            calc.update(1000, 0);
        }

        let rms = calc.get_rms();
        // sqrt(mean((1000² + 0²)/2)) = sqrt(500000) ≈ 707
        assert!(rms > 700.0 && rms < 715.0);
    }

    #[test]
    fn rms_calculator_warmup() {
        let mut calc = RmsCalculator::new();
        assert!(!calc.is_warmed_up());

        for _ in 0..99 {
            calc.update(1000, 1000);
        }
        assert!(!calc.is_warmed_up());

        calc.update(1000, 1000);
        assert!(calc.is_warmed_up());
    }

    #[test]
    fn rms_calculator_reset() {
        let mut calc = RmsCalculator::new();

        for _ in 0..100 {
            calc.update(1000, 1000);
        }
        assert!(calc.is_warmed_up());
        assert!(calc.get_rms() > 900.0);

        calc.reset();
        assert!(!calc.is_warmed_up());
        assert_eq!(calc.get_rms(), 0.0);
    }
}
