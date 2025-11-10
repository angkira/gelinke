use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::peripherals::{ADC1, DMA1_CH1};
use embassy_stm32::Peripherals;

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

/// Current and voltage sensor using ADC1 with DMA.
///
/// CLN17 V2.0 hardware connections:
/// - PA3 (ADC1_IN4): DRV8844 AISEN (Phase A current)
/// - PB0 (ADC1_IN15): DRV8844 BISEN (Phase B current)
/// - PA2 (ADC1_IN3): Vbus voltage divider
pub struct Sensors {
    adc: Adc<'static, ADC1>,
    dma: embassy_stm32::Peri<'static, DMA1_CH1>,
    current_a: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    current_b: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    vbus: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    buffer: [u16; 3],
}

impl Sensors {
    /// Create a new sensor instance.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    pub fn new(p: Peripherals) -> Self {
        let adc = Adc::new(p.ADC1);

        // Current sensing from DRV8844
        let current_a = p.PA3.degrade_adc();  // ADC1_IN4
        let current_b = p.PB0.degrade_adc();  // ADC1_IN15

        // Supply voltage monitoring
        let vbus = p.PA2.degrade_adc();  // ADC1_IN3

        let dma = p.DMA1_CH1;

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
}

// Legacy compatibility aliases
pub type CurrentSensors = Sensors;

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
}
