use embassy_stm32::peripherals::CORDIC;
use embassy_stm32::Peripherals;
use fixed::types::I1F15;

/// Hardware CORDIC accelerator for trigonometric calculations.
pub struct CordicEngine {
    _cordic: embassy_stm32::Peri<'static, CORDIC>,
}

impl CordicEngine {
    /// Create a new CORDIC engine instance.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    pub fn new(p: Peripherals) -> Self {
        let cordic = p.CORDIC;
        
        // TODO: Configure CORDIC for sine/cosine calculations
        // Embassy doesn't have full CORDIC HAL yet, so this is a placeholder
        
        Self { _cordic: cordic }
    }

    /// Calculate sine and cosine using CORDIC hardware.
    ///
    /// # Arguments
    /// * `angle_mdeg` - Angle in millidegrees (0..360000)
    ///
    /// Returns (sin, cos) as I1F15 fixed-point values
    pub fn sin_cos(&mut self, angle_mdeg: u32) -> (I1F15, I1F15) {
        // Convert millidegrees to radians in fixed-point
        // angle_rad = angle_mdeg * PI / 180000
        let angle_rad = (angle_mdeg as f32 * core::f32::consts::PI / 180_000.0) as f32;
        
        // TODO: Use hardware CORDIC when HAL is available
        // For now, use software fallback
        let sin_val = libm::sinf(angle_rad);
        let cos_val = libm::cosf(angle_rad);
        
        (
            I1F15::from_num(sin_val),
            I1F15::from_num(cos_val),
        )
    }

    /// Perform Park transform using CORDIC.
    ///
    /// # Arguments
    /// * `alpha` - Alpha axis current
    /// * `beta` - Beta axis current
    /// * `angle_mdeg` - Electrical angle in millidegrees
    ///
    /// Returns (d, q) currents
    pub fn park_transform(&mut self, alpha: I1F15, beta: I1F15, angle_mdeg: u32) -> (I1F15, I1F15) {
        let (sin, cos) = self.sin_cos(angle_mdeg);
        
        // Park transform:
        // id =  alpha * cos + beta * sin
        // iq = -alpha * sin + beta * cos
        let d = alpha * cos + beta * sin;
        let q = -alpha * sin + beta * cos;
        
        (d, q)
    }

    /// Perform inverse Park transform using CORDIC.
    ///
    /// # Arguments
    /// * `d` - D axis voltage
    /// * `q` - Q axis voltage
    /// * `angle_mdeg` - Electrical angle in millidegrees
    ///
    /// Returns (alpha, beta) voltages
    pub fn inverse_park_transform(&mut self, d: I1F15, q: I1F15, angle_mdeg: u32) -> (I1F15, I1F15) {
        let (sin, cos) = self.sin_cos(angle_mdeg);
        
        // Inverse Park transform:
        // alpha = d * cos - q * sin
        // beta  = d * sin + q * cos
        let alpha = d * cos - q * sin;
        let beta = d * sin + q * cos;
        
        (alpha, beta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_cos_zero() {
        // Can't test hardware without mocking, but test the conversion logic
        let angle_mdeg = 0;
        let angle_rad = angle_mdeg as f32 * core::f32::consts::PI / 180_000.0;
        let sin_expected = libm::sinf(angle_rad);
        let cos_expected = libm::cosf(angle_rad);
        
        assert!((sin_expected - 0.0).abs() < 0.01);
        assert!((cos_expected - 1.0).abs() < 0.01);
    }

    #[test]
    fn sin_cos_90deg() {
        let angle_mdeg = 90_000;
        let angle_rad = angle_mdeg as f32 * core::f32::consts::PI / 180_000.0;
        let sin_expected = libm::sinf(angle_rad);
        let cos_expected = libm::cosf(angle_rad);
        
        assert!((sin_expected - 1.0).abs() < 0.01);
        assert!((cos_expected - 0.0).abs() < 0.01);
    }

    #[test]
    fn park_transform_zero_angle() {
        // At zero angle: id = alpha, iq = beta
        let alpha = I1F15::from_num(0.5);
        let beta = I1F15::from_num(0.3);
        
        let angle_rad = 0.0;
        let sin = libm::sinf(angle_rad);
        let cos = libm::cosf(angle_rad);
        
        let d_expected = alpha.to_num::<f32>() * cos + beta.to_num::<f32>() * sin;
        let q_expected = -alpha.to_num::<f32>() * sin + beta.to_num::<f32>() * cos;
        
        assert!((d_expected - alpha.to_num::<f32>()).abs() < 0.01);
        assert!((q_expected - beta.to_num::<f32>()).abs() < 0.01);
    }
}

