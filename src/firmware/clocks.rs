use embassy_stm32::peripherals::RCC;
use embassy_stm32::rcc::{
    self, AHBPrescaler, APBPrescaler, Config as RccConfig, Pll, PllPreDiv, PllRDiv, PllSource, Sysclk,
};

use crate::firmware::config::SYSCLK_HZ;

pub fn rcc_config() -> RccConfig {
    let mut cfg = RccConfig::default();

    cfg.hsi = true;
    cfg.sys = Sysclk::PLL1_R;
    cfg.pll = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV4,
        mul: rcc::PllMul::MUL85,
        divp: None,
        divq: None,
        divr: Some(PllRDiv::DIV2),
    });
    cfg.ahb_pre = AHBPrescaler::DIV1;
    cfg.apb1_pre = APBPrescaler::DIV1;
    cfg.apb2_pre = APBPrescaler::DIV1;
    cfg.boost = true;

    cfg
}

pub fn log_clocks(rcc: &embassy_stm32::Peri<'_, RCC>) {
    let clk = rcc::clocks(rcc);
    if let Some(sys) = clk.sys.to_hertz() {
        defmt::info!("sysclk={} Hz", sys.0);
        if sys.0 != SYSCLK_HZ {
            defmt::warn!("sysclk mismatch: actual {} Hz, expected {} Hz", sys.0, SYSCLK_HZ);
        }
    } else {
        defmt::warn!("sysclk not configured");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pll_configuration_matches_expected() {
        let cfg = rcc_config();
        assert!(cfg.hsi);
        assert!(matches!(cfg.sys, Sysclk::PLL1_R));
        let pll = cfg.pll.expect("pll");
        assert!(matches!(pll.source, PllSource::HSI));
        assert!(matches!(pll.prediv, PllPreDiv::DIV4));
        assert!(matches!(pll.mul, rcc::PllMul::MUL85));
        assert!(matches!(pll.divr, Some(PllRDiv::DIV2)));
    }
}
