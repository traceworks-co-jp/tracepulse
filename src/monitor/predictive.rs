use crate::db::models::InterfaceSample;

pub const ERROR_RATIO_WARNING: f64 = 0.00001;
pub const RX_POWER_WARNING_DBM: f64 = -18.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictiveIndicators {
    pub error_ratio: f64,
    pub error_acceleration: f64,
    pub rx_optical_power_dbm: Option<f64>,
    pub error_ratio_warning: bool,
    pub trend_warning: bool,
    pub dom_warning: bool,
}

pub fn evaluate_predictive(samples: &[InterfaceSample]) -> Option<PredictiveIndicators> {
    let latest = samples.first()?;
    let previous = samples.get(1)?;
    let older = samples.get(2);
    let latest_errors = counter_delta(previous.in_errors, latest.in_errors);
    let latest_packets = counter_delta(
        previous.in_packets.saturating_add(previous.out_packets),
        latest.in_packets.saturating_add(latest.out_packets),
    );
    let error_ratio = if latest_packets == 0 {
        0.0
    } else {
        latest_errors as f64 / latest_packets as f64
    };
    let previous_errors = older
        .map(|sample| counter_delta(sample.in_errors, previous.in_errors))
        .unwrap_or(0);
    let error_acceleration = latest_errors as f64 - previous_errors as f64;
    let rx_optical_power_dbm = latest.rx_optical_power_dbm;

    Some(PredictiveIndicators {
        error_ratio,
        error_acceleration,
        rx_optical_power_dbm,
        error_ratio_warning: error_ratio > ERROR_RATIO_WARNING,
        trend_warning: older.is_some() && error_acceleration > 0.0,
        dom_warning: rx_optical_power_dbm.is_some_and(|value| value <= RX_POWER_WARNING_DBM),
    })
}

pub fn counter_delta(previous: u64, latest: u64) -> u64 {
    if latest >= previous {
        latest - previous
    } else {
        (u32::MAX as u64 - previous) + latest + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ratio_above_point_zero_zero_one_percent() {
        let mut latest = sample(102, 100_000);
        latest.in_errors = 102;
        latest.in_packets = 100_000;
        let previous = sample(100, 0);
        let result = evaluate_predictive(&[latest, previous]).unwrap();
        assert!(result.error_ratio_warning);
    }

    #[test]
    fn detects_positive_error_acceleration() {
        let mut latest = sample(120, 1000);
        latest.in_errors = 130;
        let previous = sample(110, 1000);
        let older = sample(100, 1000);
        let result = evaluate_predictive(&[latest, previous, older]).unwrap();
        assert!(result.trend_warning);
    }

    fn sample(errors: u64, packets: u64) -> InterfaceSample {
        InterfaceSample {
            id: None,
            device_id: 1,
            if_index: 1,
            if_name: "Gi1/0/1".to_string(),
            link_status: "up".to_string(),
            in_errors: errors,
            out_errors: 0,
            in_packets: packets,
            out_packets: 0,
            in_discards: 0,
            out_discards: 0,
            late_collisions: 0,
            fcs_errors: 0,
            alignment_errors: 0,
            frame_too_longs: 0,
            internal_mac_receive_errors: 0,
            rx_optical_power_dbm: None,
            in_octets: 0,
            out_octets: 0,
            bandwidth_utilization: 0.0,
            sampled_at: String::new(),
        }
    }
}
