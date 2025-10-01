pub struct PhasePwm;

impl PhasePwm {
    pub fn new() -> Self {
        Self
    }

    pub fn set_duties(&mut self, _duty: [u16; 3]) {}
}
