pub fn calculate_bandwidth_utilization(used: u64, capacity: u64) -> f64 {
    if capacity == 0 {
        return 0.0;
    }
    ((used as f64 / capacity as f64) * 100.0).clamp(0.0, 100.0)
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
    ((throughput_bps / capacity_bps as f64) * 100.0).clamp(0.0, 100.0)
}

pub fn calculate_duplex_bandwidth_utilization_from_deltas(
    prev_in_octets: u64, current_in_octets: u64,
    prev_out_octets: u64, current_out_octets: u64,
    interval_seconds: u64, capacity_bps: u64,
) -> f64 {
    if interval_seconds == 0 || capacity_bps == 0 { return 0.0; }
    let in_bps = current_in_octets.saturating_sub(prev_in_octets) as f64 * 8.0 / interval_seconds as f64;
    let out_bps = current_out_octets.saturating_sub(prev_out_octets) as f64 * 8.0 / interval_seconds as f64;
    (in_bps.max(out_bps) / capacity_bps as f64 * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_duplex_uses_greater_direction_and_never_exceeds_capacity() {
        assert_eq!(calculate_duplex_bandwidth_utilization_from_deltas(0, 5_000_000, 0, 10_000_000, 1, 1_000_000_000), 8.0);
        assert_eq!(calculate_duplex_bandwidth_utilization_from_deltas(0, 200_000_000, 0, 100_000_000, 1, 1_000_000_000), 100.0);
        assert_eq!(calculate_duplex_bandwidth_utilization_from_deltas(10, 5, 0, 0, 1, 1_000_000_000), 0.0);
        assert_eq!(calculate_bandwidth_utilization(2, 1), 100.0);
    }
}

pub fn calculate_error_rate(errors: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (errors as f64 / total as f64) * 100.0
}
