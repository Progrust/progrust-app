use super::*;

#[test]
fn roll_stays_in_range() {
    for _ in 0..200 {
        let value = dice::roll(6);
        assert!((1..=6).contains(&value));
    }
}

#[test]
fn roll_one_sided_die() {
    for _ in 0..20 {
        assert_eq!(dice::roll(1), 1);
    }
}

#[test]
fn roll_all_returns_count_values() {
    assert_eq!(game::roll_all(6, 5).len(), 5);
}

#[test]
fn roll_all_zero_count() {
    assert_eq!(game::roll_all(6, 0).len(), 0);
}

#[test]
fn roll_all_values_stay_in_range() {
    let values = game::roll_all(20, 300);
    assert_eq!(values.len(), 300);
    for value in values {
        assert!((1..=20).contains(&value));
    }
}

#[test]
fn roll_all_one_sided_die() {
    assert_eq!(game::roll_all(1, 4), vec![1, 1, 1, 1]);
}
