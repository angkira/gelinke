/// Status LED control for CLN17 V2.0
///
/// Hardware: RGB LED with active-low control
/// - PB13: Red LED (active low)
/// - PB14: Green LED (active low)
/// - PB15: Blue LED (active low)

use embassy_stm32::gpio::{Level, Output, Speed};

/// RGB status LED controller.
pub struct StatusLeds {
    red: Output<'static>,
    green: Output<'static>,
    blue: Output<'static>,
}

/// Predefined LED colors for common status indications.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LedColor {
    Off,
    Red,
    Green,
    Blue,
    Yellow,    // Red + Green
    Cyan,      // Green + Blue
    Magenta,   // Red + Blue
    White,     // All on
}

impl StatusLeds {
    /// Create a new status LED controller.
    ///
    /// # Arguments
    /// * `pb13` - PB13 pin for Red LED
    /// * `pb14` - PB14 pin for Green LED
    /// * `pb15` - PB15 pin for Blue LED
    ///
    /// # Initial State
    /// All LEDs off (pins high, active low)
    pub fn new(
        pb13: embassy_stm32::Peri<'static, impl embassy_stm32::gpio::Pin>,
        pb14: embassy_stm32::Peri<'static, impl embassy_stm32::gpio::Pin>,
        pb15: embassy_stm32::Peri<'static, impl embassy_stm32::gpio::Pin>,
    ) -> Self {
        // Active low: HIGH = off, LOW = on
        let red = Output::new(pb13, Level::High, Speed::Low);
        let green = Output::new(pb14, Level::High, Speed::Low);
        let blue = Output::new(pb15, Level::High, Speed::Low);

        Self { red, green, blue }
    }

    /// Turn on red LED.
    pub fn red_on(&mut self) {
        self.red.set_low();
    }

    /// Turn off red LED.
    pub fn red_off(&mut self) {
        self.red.set_high();
    }

    /// Turn on green LED.
    pub fn green_on(&mut self) {
        self.green.set_low();
    }

    /// Turn off green LED.
    pub fn green_off(&mut self) {
        self.green.set_high();
    }

    /// Turn on blue LED.
    pub fn blue_on(&mut self) {
        self.blue.set_low();
    }

    /// Turn off blue LED.
    pub fn blue_off(&mut self) {
        self.blue.set_high();
    }

    /// Turn all LEDs off.
    pub fn all_off(&mut self) {
        self.red.set_high();
        self.green.set_high();
        self.blue.set_high();
    }

    /// Set LED color using predefined colors.
    pub fn set_color(&mut self, color: LedColor) {
        match color {
            LedColor::Off => self.all_off(),
            LedColor::Red => {
                self.red_on();
                self.green_off();
                self.blue_off();
            }
            LedColor::Green => {
                self.red_off();
                self.green_on();
                self.blue_off();
            }
            LedColor::Blue => {
                self.red_off();
                self.green_off();
                self.blue_on();
            }
            LedColor::Yellow => {
                self.red_on();
                self.green_on();
                self.blue_off();
            }
            LedColor::Cyan => {
                self.red_off();
                self.green_on();
                self.blue_on();
            }
            LedColor::Magenta => {
                self.red_on();
                self.green_off();
                self.blue_on();
            }
            LedColor::White => {
                self.red_on();
                self.green_on();
                self.blue_on();
            }
        }
    }

    /// Set RGB values directly.
    ///
    /// # Arguments
    /// * `r` - Red state (true = on, false = off)
    /// * `g` - Green state
    /// * `b` - Blue state
    pub fn set_rgb(&mut self, r: bool, g: bool, b: bool) {
        if r {
            self.red_on();
        } else {
            self.red_off();
        }
        if g {
            self.green_on();
        } else {
            self.green_off();
        }
        if b {
            self.blue_on();
        } else {
            self.blue_off();
        }
    }

    /// Toggle red LED.
    pub fn toggle_red(&mut self) {
        self.red.toggle();
    }

    /// Toggle green LED.
    pub fn toggle_green(&mut self) {
        self.green.toggle();
    }

    /// Toggle blue LED.
    pub fn toggle_blue(&mut self) {
        self.blue.toggle();
    }

    /// Indicate system status with color.
    ///
    /// Common usage:
    /// - Green: Running normally
    /// - Yellow: Warning
    /// - Red: Error/fault
    /// - Blue: Idle/standby
    pub fn indicate_status(&mut self, status: SystemStatus) {
        match status {
            SystemStatus::Idle => self.set_color(LedColor::Blue),
            SystemStatus::Running => self.set_color(LedColor::Green),
            SystemStatus::Warning => self.set_color(LedColor::Yellow),
            SystemStatus::Error => self.set_color(LedColor::Red),
            SystemStatus::Fault => {
                // Blink red (caller should handle timing)
                self.set_color(LedColor::Red);
            }
        }
    }
}

/// System status levels for LED indication.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SystemStatus {
    Idle,
    Running,
    Warning,
    Error,
    Fault,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn led_colors() {
        // Verify color enum is properly defined
        assert_eq!(LedColor::Off, LedColor::Off);
        assert_ne!(LedColor::Red, LedColor::Green);
    }

    #[test]
    fn system_status() {
        // Verify status enum
        assert_eq!(SystemStatus::Running, SystemStatus::Running);
        assert_ne!(SystemStatus::Error, SystemStatus::Warning);
    }
}
