/// STM32 Internal Flash Storage for Persistent Data.
///
/// Uses last 2 pages of flash (2 KB each) for dual-bank storage:
/// - Bank A (primary): Page 62 @ 0x0801F000
/// - Bank B (backup): Page 63 @ 0x0801F800
///
/// Dual-bank provides redundancy and safe updates.

use embassy_stm32::flash::{Flash, Blocking};
use crc::{Crc, CRC_32_CKSUM};

const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

/// Flash storage layout (2048 bytes per bank).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StoredData {
    /// Magic number for validity check (0xCAFEBABE).
    pub magic: u32,

    /// Data version (for migration).
    pub version: u32,

    /// Motor calibration results.
    pub calibration: CalibrationData,

    /// User configuration.
    pub config: UserConfig,

    /// Diagnostic data.
    pub diagnostics: DiagnosticData,

    /// Factory data (read-only after manufacturing).
    pub factory: FactoryData,

    /// Reserved for future use (512 bytes).
    pub reserved: [u8; 512],

    /// CRC32 checksum (must be last field).
    pub crc: u32,
}

/// Motor calibration data.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CalibrationData {
    pub valid: bool,
    pub inertia_kg_m2: f32,
    pub friction_static_nm: f32,
    pub friction_viscous_nm_per_rad_s: f32,
    pub torque_constant_nm_per_a: f32,
    pub phase_resistance_ohm: f32,
    pub phase_inductance_h: f32,
    pub encoder_offset_counts: i32,
    pub timestamp_s: u32,  // Unix timestamp
    pub reserved: [u8; 16],
}

/// User configuration.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UserConfig {
    pub motor_poles: u8,
    pub motor_rated_current_ma: u16,
    pub max_current_ma: u16,
    pub max_voltage_mv: u32,
    pub max_temperature_c: u8,
    pub can_node_id: u8,
    pub position_kp: f32,
    pub position_ki: f32,
    pub position_kd: f32,
    pub velocity_kp: f32,
    pub velocity_ki: f32,
    pub reserved: [u8; 32],
}

/// Diagnostic data.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DiagnosticData {
    pub total_runtime_hours: u32,
    pub power_cycle_count: u32,
    pub fault_history: [FaultRecord; 10],
    pub temp_max_c: i16,
    pub temp_min_c: i16,
    pub last_fault_timestamp: u32,
    pub reserved: [u8; 64],
}

/// Individual fault record.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FaultRecord {
    pub fault_type: u8,  // FaultType enum as u8
    pub timestamp: u32,
    pub vbus_mv: u16,
    pub current_ma: u16,
    pub temp_c: i8,
    pub reserved: [u8; 7],
}

/// Factory data (set during manufacturing).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FactoryData {
    pub hardware_version: u16,
    pub serial_number: u32,
    pub manufacturing_date: u32,  // Unix timestamp
    pub factory_test_passed: bool,
    pub reserved: [u8; 32],
}

/// Flash storage manager.
pub struct FlashStorage {
    flash: Flash<'static, Blocking>,
}

impl FlashStorage {
    const BANK_A_ADDR: u32 = 0x0801_F000;  // Page 62
    const BANK_B_ADDR: u32 = 0x0801_F800;  // Page 63
    const PAGE_SIZE: usize = 2048;
    const MAGIC: u32 = 0xCAFE_BABE;
    const VERSION: u32 = 1;

    /// Create a new flash storage instance.
    pub fn new(flash_peripheral: embassy_stm32::Peri<'static, embassy_stm32::peripherals::FLASH>) -> Self {
        let flash = Flash::new_blocking(flash_peripheral);

        defmt::info!("Flash storage initialized");
        defmt::info!("  Bank A: 0x{:08X}", Self::BANK_A_ADDR);
        defmt::info!("  Bank B: 0x{:08X}", Self::BANK_B_ADDR);

        Self { flash }
    }

    /// Load data from flash (tries Bank A, then Bank B if corrupted).
    pub fn load(&mut self) -> Result<StoredData, StorageError> {
        // Try Bank A first
        match self.load_bank(Self::BANK_A_ADDR) {
            Ok(data) => {
                defmt::info!("Loaded data from Bank A");
                return Ok(data);
            }
            Err(e) => {
                defmt::warn!("Bank A invalid: {:?}, trying Bank B", e);
            }
        }

        // Try Bank B
        match self.load_bank(Self::BANK_B_ADDR) {
            Ok(data) => {
                defmt::info!("Loaded data from Bank B");
                return Ok(data);
            }
            Err(e) => {
                defmt::error!("Bank B also invalid: {:?}", e);
                return Err(e);
            }
        }
    }

    /// Save data to both banks for redundancy.
    pub fn save(&mut self, data: &StoredData) -> Result<(), StorageError> {
        // Calculate CRC
        let mut data_with_crc = *data;
        data_with_crc.crc = self.calculate_crc(data);

        // Erase and write Bank A
        self.erase_page(Self::BANK_A_ADDR)?;
        self.write_bank(Self::BANK_A_ADDR, &data_with_crc)?;

        // Verify Bank A
        let _ = self.load_bank(Self::BANK_A_ADDR)?;

        // Erase and write Bank B
        self.erase_page(Self::BANK_B_ADDR)?;
        self.write_bank(Self::BANK_B_ADDR, &data_with_crc)?;

        defmt::info!("Saved data to both banks");
        Ok(())
    }

    /// Create default data structure.
    pub fn create_default() -> StoredData {
        StoredData {
            magic: Self::MAGIC,
            version: Self::VERSION,
            calibration: CalibrationData::default(),
            config: UserConfig::default(),
            diagnostics: DiagnosticData::default(),
            factory: FactoryData::default(),
            reserved: [0; 512],
            crc: 0,  // Will be calculated on save
        }
    }

    fn calculate_crc(&self, data: &StoredData) -> u32 {
        // CRC over everything except the CRC field itself
        let bytes = unsafe {
            core::slice::from_raw_parts(
                data as *const _ as *const u8,
                core::mem::size_of::<StoredData>() - 4,  // Exclude CRC field
            )
        };

        CRC.checksum(bytes)
    }

    fn load_bank(&mut self, addr: u32) -> Result<StoredData, StorageError> {
        // Read data from flash
        let data = unsafe {
            let ptr = addr as *const StoredData;
            *ptr
        };

        // Validate magic number
        if data.magic != Self::MAGIC {
            return Err(StorageError::InvalidMagic);
        }

        // Validate version
        if data.version != Self::VERSION {
            return Err(StorageError::VersionMismatch);
        }

        // Validate CRC
        let stored_crc = data.crc;
        let calculated_crc = self.calculate_crc(&data);
        if stored_crc != calculated_crc {
            return Err(StorageError::CrcMismatch);
        }

        Ok(data)
    }

    fn erase_page(&mut self, addr: u32) -> Result<(), StorageError> {
        // STM32G4 flash erase by page
        self.flash.blocking_erase(addr, addr + Self::PAGE_SIZE as u32)
            .map_err(|_| StorageError::EraseError)?;
        Ok(())
    }

    fn write_bank(&mut self, addr: u32, data: &StoredData) -> Result<(), StorageError> {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                data as *const _ as *const u8,
                core::mem::size_of::<StoredData>(),
            )
        };

        self.flash.blocking_write(addr, bytes)
            .map_err(|_| StorageError::WriteError)?;
        Ok(())
    }
}

/// Storage errors.
#[derive(Debug, Clone, Copy, defmt::Format, PartialEq)]
pub enum StorageError {
    InvalidMagic,
    VersionMismatch,
    CrcMismatch,
    EraseError,
    WriteError,
}

// Default implementations
impl Default for StoredData {
    fn default() -> Self {
        Self {
            magic: FlashStorage::MAGIC,
            version: FlashStorage::VERSION,
            calibration: CalibrationData::default(),
            config: UserConfig::default(),
            diagnostics: DiagnosticData::default(),
            factory: FactoryData::default(),
            reserved: [0; 512],
            crc: 0,
        }
    }
}

impl Default for CalibrationData {
    fn default() -> Self {
        Self {
            valid: false,
            inertia_kg_m2: 0.0,
            friction_static_nm: 0.0,
            friction_viscous_nm_per_rad_s: 0.0,
            torque_constant_nm_per_a: 0.0,
            phase_resistance_ohm: 0.0,
            phase_inductance_h: 0.0,
            encoder_offset_counts: 0,
            timestamp_s: 0,
            reserved: [0; 16],
        }
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            motor_poles: 8,
            motor_rated_current_ma: 1750,
            max_current_ma: 2000,
            max_voltage_mv: 48000,
            max_temperature_c: 85,
            can_node_id: 1,
            position_kp: 100.0,
            position_ki: 0.0,
            position_kd: 10.0,
            velocity_kp: 1.0,
            velocity_ki: 0.1,
            reserved: [0; 32],
        }
    }
}

impl Default for DiagnosticData {
    fn default() -> Self {
        Self {
            total_runtime_hours: 0,
            power_cycle_count: 0,
            fault_history: [FaultRecord::default(); 10],
            temp_max_c: -40,
            temp_min_c: 125,
            last_fault_timestamp: 0,
            reserved: [0; 64],
        }
    }
}

impl Default for FaultRecord {
    fn default() -> Self {
        Self {
            fault_type: 0,
            timestamp: 0,
            vbus_mv: 0,
            current_ma: 0,
            temp_c: 0,
            reserved: [0; 7],
        }
    }
}

impl Default for FactoryData {
    fn default() -> Self {
        Self {
            hardware_version: 200,  // CLN17 v2.0
            serial_number: 0,
            manufacturing_date: 0,
            factory_test_passed: false,
            reserved: [0; 32],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_data_size() {
        // Should fit in 2048 bytes (one flash page)
        assert!(core::mem::size_of::<StoredData>() <= 2048);
    }

    #[test]
    fn default_values() {
        let data = StoredData::default();
        assert_eq!(data.magic, FlashStorage::MAGIC);
        assert_eq!(data.version, FlashStorage::VERSION);
        assert!(!data.calibration.valid);
    }

    #[test]
    fn user_config_defaults() {
        let config = UserConfig::default();
        assert_eq!(config.motor_poles, 8);
        assert_eq!(config.max_current_ma, 2000);
        assert_eq!(config.can_node_id, 1);
    }
}
