pub mod can_comm;
pub mod foc;

#[cfg(feature = "renode-mock")]
pub mod mock_can;

#[cfg(feature = "renode-mock")]
pub mod mock_foc;
pub mod telemetry;
