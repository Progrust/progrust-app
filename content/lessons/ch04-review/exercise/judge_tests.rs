use super::*;

#[test]
fn go_up_adds_parents() {
    assert_eq!(go_up(String::from("logs/app"), 3), String::from("logs/app/../../.."));
}

#[test]
fn go_up_zero_levels() {
    assert_eq!(go_up(String::from("logs/app"), 0), String::from("logs/app"));
}

#[test]
fn go_up_one_level() {
    assert_eq!(go_up(String::from("src"), 1), String::from("src/.."));
}

#[test]
fn path_and_up_keeps_path() {
    assert_eq!(
        path_and_up(String::from("data"), 2),
        (String::from("data"), String::from("data/../.."))
    );
}

#[test]
fn path_and_up_zero_levels() {
    assert_eq!(
        path_and_up(String::from("tmp"), 0),
        (String::from("tmp"), String::from("tmp"))
    );
}

#[test]
fn diff_args_joins_with_space() {
    assert_eq!(
        diff_args(String::from("a/b"), 1),
        String::from("a/b a/b/..")
    );
}

#[test]
fn diff_args_zero_levels() {
    assert_eq!(
        diff_args(String::from("a/b"), 0),
        String::from("a/b a/b")
    );
}
