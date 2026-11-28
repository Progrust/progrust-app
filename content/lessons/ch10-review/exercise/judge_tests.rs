use super::*;

#[test]
fn delay_stays_in_range() {
    for _ in 0..200 {
        let value = jitter::delay_ms(500);
        assert!((0..=500).contains(&value));
    }
}

#[test]
fn delay_zero_max() {
    for _ in 0..20 {
        assert_eq!(jitter::delay_ms(0), 0);
    }
}

#[test]
fn plan_delays_returns_count_values() {
    assert_eq!(retry::plan_delays(500, 5).len(), 5);
}

#[test]
fn plan_delays_zero_count() {
    assert_eq!(retry::plan_delays(500, 0).len(), 0);
}

#[test]
fn plan_delays_values_stay_in_range() {
    let values = retry::plan_delays(1000, 300);
    assert_eq!(values.len(), 300);
    for value in values {
        assert!((0..=1000).contains(&value));
    }
}

#[test]
fn plan_delays_zero_max() {
    assert_eq!(retry::plan_delays(0, 4), vec![0, 0, 0, 0]);
}
