fn exclaim(text: String, count: u32) -> String {
    let mut result = text;
    let mut added = 0;
    while added < count {
        result.push_str("!");
        added = added + 1;
    }
    result
}

fn plain_and_loud(text: String, count: u32) -> (String, String) {
    let loud = exclaim(text.clone(), count);
    (text, loud)
}

fn call_twice(text: String, count: u32) -> String {
    let first = exclaim(text.clone(), count);
    let second = exclaim(text, count);
    format!("{first} {second}")
}

fn main() {
    println!("{}", exclaim(String::from("がんばれ"), 3));

    let (plain, loud) = plain_and_loud(String::from("Alice"), 2);
    println!("{plain}");
    println!("{loud}");

    println!("{}", call_twice(String::from("ファイト"), 2));
}
