use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::peripherals::{ADC1, DMA1_CH1};
use embassy_stm32::Peripherals;

/// ADC configuration for current sensing.
pub const ADC_SAMPLE_TIME: SampleTime = SampleTime::CYCLES12_5;

/// Shunt resistor value in milliohms.
pub const SHUNT_RESISTANCE_MOHM: u32 = 10;

/// Current sense amplifier gain.
pub const CURRENT_SENSE_GAIN: u32 = 20;

/// ADC reference voltage in millivolts.
pub const VREF_MV: u32 = 3300;

/// Two-channel current sensor using ADC1 with DMA.
pub struct CurrentSensors {
    adc: Adc<'static, ADC1>,
    dma: embassy_stm32::Peri<'static, DMA1_CH1>,
    phase_a: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    phase_b: embassy_stm32::adc::AnyAdcChannel<ADC1>,
    buffer: [u16; 2],
}

impl CurrentSensors {
    /// Create a new current sensor instance.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    pub fn new(p: Peripherals) -> Self {
        let adc = Adc::new(p.ADC1);
        let phase_a = p.PA0.degrade_adc();
        let phase_b = p.PA1.degrade_adc();
        let dma = p.DMA1_CH1;
        
        Self {
            adc,
            dma,
            phase_a,
            phase_b,
            buffer: [0; 2],
        }
    }

    /// Read raw ADC values from both phase current sensors.
    ///
    /// Returns [phase_a_raw, phase_b_raw]
    pub async fn read_raw(&mut self) -> [u16; 2] {
        self.adc
            .read(
                self.dma.reborrow(),
                [
                    (&mut self.phase_a, ADC_SAMPLE_TIME),
                    (&mut self.phase_b, ADC_SAMPLE_TIME),
                ]
                .into_iter(),
                &mut self.buffer,
            )
            .await;
        
        self.buffer
    }

    /// Convert raw ADC value to current in milliamps.
    ///
    /// # Arguments
    /// * `raw` - Raw ADC value (0-4095 for 12-bit ADC)
    /// * `offset` - Calibrated zero-current ADC offset
    pub fn raw_to_milliamps(raw: u16, offset: u16) -> i32 {
        let voltage_mv = (raw as i32 * VREF_MV as i32) / 4096;
        let offset_mv = (offset as i32 * VREF_MV as i32) / 4096;
        let diff_mv = voltage_mv - offset_mv;
        
        // I = V / (R * G)
        (diff_mv * 1000) / (SHUNT_RESISTANCE_MOHM as i32 * CURRENT_SENSE_GAIN as i32)
    }

    /// Calibrate ADC offsets by averaging multiple samples at zero current.
    ///
    /// Returns [phase_a_offset, phase_b_offset]
    pub async fn calibrate_offsets(&mut self, samples: usize) -> [u16; 2] {
        let mut sum_a: u32 = 0;
        let mut sum_b: u32 = 0;

        for _ in 0..samples {
            let [a, b] = self.read_raw().await;
            sum_a += a as u32;
            sum_b += b as u32;
        }

        [
            (sum_a / samples as u32) as u16,
            (sum_b / samples as u32) as u16,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_zero_current() {
        // At zero current, raw should equal offset
        let offset = 2048;
        let current = CurrentSensors::raw_to_milliamps(2048, offset);
        assert_eq!(current, 0);
    }

    #[test]
    fn conversion_positive_current() {
        // Test positive current conversion
        let offset = 2048;
        let raw = 2548; // +500 counts above offset
        let current = CurrentSensors::raw_to_milliamps(raw, offset);
        // Approximate check: should be positive
        assert!(current > 0);
    }

    #[test]
    fn conversion_negative_current() {
        // Test negative current conversion
        let offset = 2048;
        let raw = 1548; // -500 counts below offset
        let current = CurrentSensors::raw_to_milliamps(raw, offset);
        // Should be negative
        assert!(current < 0);
    }
}
