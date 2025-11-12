/// Power Telemetry Task
///
/// Streams power monitoring data over iRPC/CAN at configurable rates.
/// Provides real-time visibility into power consumption, thermal state,
/// and fault conditions.

use embassy_time::{Duration, Ticker};

use crate::firmware::tasks::power_monitor::POWER_METRICS;

/// Power telemetry streaming task.
///
/// **Features:**
/// - Configurable rate (1-100 Hz, default 10 Hz)
/// - Reads from shared POWER_METRICS state
/// - Minimal overhead (async/await, no blocking)
/// - Automatic fault/emergency notifications
///
/// **Telemetry Data:**
/// - Voltage, current (RMS + instantaneous)
/// - Power, energy, charge accumulation
/// - MCU temperature, thermal throttle status
/// - Fault counters and event history
///
/// **Integration:**
/// This task reads from `POWER_METRICS` static and would send
/// via iRPC transport. Since iRPC PowerMetrics message support
/// is pending, this currently just logs the data.
///
/// # Arguments
/// * `rate_hz` - Telemetry streaming rate (1-100 Hz)
#[embassy_executor::task]
pub async fn power_telemetry(rate_hz: u8) {
    let rate_hz = rate_hz.clamp(1, 100);
    let period_ms = 1000 / rate_hz as u64;

    defmt::info!("Power telemetry task starting @ {} Hz", rate_hz);

    let mut ticker = Ticker::every(Duration::from_millis(period_ms));

    loop {
        ticker.next().await;

        // Read current power metrics
        let metrics = POWER_METRICS.lock().await;

        // TODO: Send via iRPC when PowerMetrics message is available
        // For now, periodic logging at reduced rate for visibility
        defmt::trace!(
            "Power: V={} mV, I_RMS={} mA, P={} mW, T={}°C, Throttle={}%",
            metrics.vbus_mv,
            metrics.i_rms_ma as u32,
            metrics.power_mw,
            metrics.mcu_temp_c,
            (metrics.throttle_factor * 100.0) as u32
        );

        // Check for faults to report
        if metrics.faults.total() > 0 {
            defmt::warn!(
                "Faults: OC={}, OV={}, UV={}, OT={}, DRV={}, ESTOP={}",
                metrics.faults.overcurrent_events,
                metrics.faults.overvoltage_events,
                metrics.faults.undervoltage_events,
                metrics.faults.overtemp_events,
                metrics.faults.driver_fault_events,
                metrics.faults.emergency_stops
            );
        }

        drop(metrics);

        // TODO: When iRPC PowerMetrics is available:
        // let msg = create_power_metrics_message(&metrics);
        // transport.send_message(&msg).await;
    }
}

/// Create iRPC PowerMetrics message (placeholder for when iRPC supports it).
///
/// This function shows the intended integration pattern.
/// Once iRPC library adds PowerMetrics message type, this will serialize
/// and send the data over CAN-FD.
///
/// # Example Integration
/// ```rust,ignore
/// use irpc::protocol::{Message, Payload, PowerMetricsPayload};
///
/// fn create_power_metrics_message(metrics: &PowerMetrics) -> Message {
///     Message {
///         header: Header {
///             msg_id: MSG_ID_POWER_METRICS,
///             timestamp: get_timestamp_ms(),
///         },
///         payload: Payload::PowerMetrics(PowerMetricsPayload {
///             vbus_mv: metrics.vbus_mv,
///             ia_ma: metrics.ia_ma,
///             ib_ma: metrics.ib_ma,
///             i_rms_ma: metrics.i_rms_ma,
///             power_mw: metrics.power_mw,
///             mcu_temp_c: metrics.mcu_temp_c,
///             throttle_factor: metrics.throttle_factor,
///             energy_mwh: metrics.energy_mwh,
///             charge_mah: metrics.charge_mah,
///             active_time_ms: metrics.active_time_ms,
///             faults: metrics.faults.clone(),
///         }),
///     }
/// }
/// ```
#[allow(dead_code)]
fn create_power_metrics_message_placeholder() {
    // Placeholder - will be implemented when iRPC supports PowerMetrics
    defmt::info!("PowerMetrics message creation pending iRPC library support");
}

/// Emergency stop notification (high priority).
///
/// This would be called from power_monitor task when emergency stop occurs.
/// Broadcasts immediately over CAN with high priority for system coordination.
///
/// # Arguments
/// * `reason` - Emergency stop reason (overvoltage, overcurrent, etc.)
/// * `vbus_mv` - Voltage at fault time
/// * `current_ma` - Current at fault time
/// * `temp_c` - Temperature at fault time
#[allow(dead_code)]
pub async fn broadcast_emergency_stop(
    _reason: &str,
    _vbus_mv: u32,
    _current_ma: i32,
    _temp_c: f32,
) {
    // TODO: Implement when iRPC EmergencyStop message is available
    defmt::error!("EMERGENCY STOP - iRPC broadcast pending");
}
