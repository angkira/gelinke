pub const SYSCLK_HZ: u32 = 170_000_000;
pub const HEARTBEAT_PERIOD_SECS: u64 = 1;

#[derive(Clone, Copy, Debug, Default)]
pub struct MotorConfig {
    pub pole_pairs: u8,
}

impl MotorConfig {
    pub const fn default() -> Self {
        Self { pole_pairs: 7 }
    }
}

#[derive(Clone, Copy)]
pub struct EncoderConfig {
    pub resolution_bits: u8,
}

impl EncoderConfig {
    pub const fn tle5012b() -> Self {
        Self {
            resolution_bits: 15,
        }
    }
}
