use embassy_stm32::gpio::Output;
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peripherals;

/// TLE5012B register addresses
const REG_ANGLE_VALUE: u16 = 0x0002;

/// TLE5012B SPI frequency (max 8 MHz)
const TLE5012B_SPI_FREQ: Hertz = Hertz(4_000_000);

/// TLE5012B 15-bit resolution
const ANGLE_RESOLUTION_BITS: u8 = 15;
const ANGLE_MAX: u16 = (1 << ANGLE_RESOLUTION_BITS) - 1;

/// TLE5012B magnetic encoder using SPI.
pub struct AngleSensor {
    spi: Spi<'static, embassy_stm32::mode::Blocking>,
    cs: Output<'static>,
}

impl AngleSensor {
    /// Create a new TLE5012B encoder instance.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    pub fn new(p: Peripherals) -> Self {
        let mut spi_config = SpiConfig::default();
        spi_config.frequency = TLE5012B_SPI_FREQ;

        let spi = Spi::new_blocking(
            p.SPI1,
            p.PA5, // SCK
            p.PA7, // MOSI
            p.PA6, // MISO
            spi_config,
        );

        let cs = Output::new(p.PC4, embassy_stm32::gpio::Level::High, embassy_stm32::gpio::Speed::VeryHigh);

        Self { spi, cs }
    }

    /// Read raw 15-bit angle value from TLE5012B.
    ///
    /// Returns angle in encoder counts (0..32767)
    pub fn read_raw_angle(&mut self) -> Result<u16, ()> {
        // TLE5012B requires 16-bit command followed by 16-bit response
        let mut tx_buf = [0u8; 4];
        let mut rx_buf = [0u8; 4];

        // Build read command: [15:15]=0 (read), [14:0]=address
        let cmd = REG_ANGLE_VALUE & 0x7FFF;
        tx_buf[0] = (cmd >> 8) as u8;
        tx_buf[1] = (cmd & 0xFF) as u8;

        self.cs.set_low();
        let result = self.spi.blocking_transfer(&mut rx_buf, &tx_buf);
        self.cs.set_high();

        if result.is_err() {
            return Err(());
        }

        // Extract 15-bit angle from response bytes 2-3
        let angle = ((rx_buf[2] as u16) << 8) | (rx_buf[3] as u16);
        let angle = angle & ANGLE_MAX;

        Ok(angle)
    }

    /// Convert raw encoder counts to electrical angle in millidegrees.
    ///
    /// # Arguments
    /// * `raw` - Raw encoder counts (0..32767)
    /// * `pole_pairs` - Motor pole pairs
    pub fn raw_to_electrical_angle_mdeg(raw: u16, pole_pairs: u8) -> u32 {
        // Mechanical angle in millidegrees
        let mech_angle_mdeg = (raw as u32 * 360_000) / (ANGLE_MAX as u32 + 1);
        
        // Electrical angle wraps pole_pairs times per revolution
        (mech_angle_mdeg * pole_pairs as u32) % 360_000
    }

    /// Read electrical angle in millidegrees.
    ///
    /// # Arguments
    /// * `pole_pairs` - Motor pole pairs
    pub fn read_electrical_angle(&mut self, pole_pairs: u8) -> Result<u32, ()> {
        let raw = self.read_raw_angle()?;
        Ok(Self::raw_to_electrical_angle_mdeg(raw, pole_pairs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_conversion_zero() {
        let electrical = AngleSensor::raw_to_electrical_angle_mdeg(0, 7);
        assert_eq!(electrical, 0);
    }

    #[test]
    fn angle_conversion_one_rev() {
        // At max counts, should wrap back close to 0
        let electrical = AngleSensor::raw_to_electrical_angle_mdeg(ANGLE_MAX, 7);
        // Should be close to 360_000 * 7 = 2_520_000, wrapped to < 360_000
        assert!(electrical < 360_000);
    }

    #[test]
    fn angle_conversion_half_rev() {
        // At half revolution mechanical
        let half_counts = ANGLE_MAX / 2;
        let electrical = AngleSensor::raw_to_electrical_angle_mdeg(half_counts, 7);
        // With 7 pole pairs, half mechanical revolution = 3.5 electrical revolutions
        // 3.5 * 360_000 = 1_260_000, wrapped to 180_000
        let expected = (180_000 * 7) % 360_000;
        assert!((electrical as i32 - expected as i32).abs() < 1000); // Within 1 degree
    }
}
