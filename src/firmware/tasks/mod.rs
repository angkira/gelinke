pub mod can_comm;
pub mod foc;
pub mod step_dir;
pub mod power_monitor;
pub mod power_telemetry;
pub mod thermal_throttle;
pub mod watchdog_feeder;

#[cfg(feature = "renode-mock")]
pub mod mock_can;

#[cfg(feature = "renode-mock")]
pub mod mock_foc;

#[cfg(feature = "renode-mock")]
pub mod mock_step_dir;

pub mod telemetry;
