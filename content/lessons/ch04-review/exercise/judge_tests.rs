use super::*;

#[test]
fn exclaim_adds_marks() {
    assert_eq!(exclaim(String::from("がんばれ"), 3), String::from("がんばれ!!!"));
}

#[test]
fn exclaim_zero_count() {
    assert_eq!(exclaim(String::from("がんばれ"), 0), String::from("がんばれ"));
}

#[test]
fn exclaim_empty_text() {
    assert_eq!(exclaim(String::from(""), 2), String::from("!!"));
}

#[test]
fn plain_and_loud_keeps_text() {
    assert_eq!(
        plain_and_loud(String::from("Alice"), 2),
        (String::from("Alice"), String::from("Alice!!"))
    );
}

#[test]
fn plain_and_loud_zero_count() {
    assert_eq!(
        plain_and_loud(String::from("太郎"), 0),
        (String::from("太郎"), String::from("太郎"))
    );
}

#[test]
fn call_twice_joins_with_space() {
    assert_eq!(
        call_twice(String::from("ファイト"), 2),
        String::from("ファイト!! ファイト!!")
    );
}

#[test]
fn call_twice_zero_count() {
    assert_eq!(
        call_twice(String::from("ファイト"), 0),
        String::from("ファイト ファイト")
    );
}
