pub fn calculate_bandwidth_utilization(used: u64, capacity: u64) -> f64 {
    if capacity == 0 {
        return 0.0;
    }
    (used as f64 / capacity as f64) * 100.0
}

pub fn calculate_bandwidth_utilization_from_delta(
    prev_octets: u64,
    current_octets: u64,
    interval_seconds: u64,
    capacity_bps: u64,
) -> f64 {
    if interval_seconds == 0 || capacity_bps == 0 {
        return 0.0;
    }

    let delta_octets = current_octets.saturating_sub(prev_octets);
    let throughput_bps = (delta_octets as f64 * 8.0) / interval_seconds as f64;
    (throughput_bps / capacity_bps as f64) * 100.0
}

pub fn calculate_error_rate(errors: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (errors as f64 / total as f64) * 100.0
}
