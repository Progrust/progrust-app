fn exclaim(text: String, count: u32) -> String {
    // ここを書く（いまは仮の値を返している）
    text
}

fn plain_and_loud(text: String, count: u32) -> (String, String) {
    // ここを書く（いまは仮の値を返している）
    (text, String::from(""))
}

fn call_twice(text: String, count: u32) -> String {
    // ここを書く（いまは仮の値を返している）
    text
}

fn main() {
    println!("{}", exclaim(String::from("がんばれ"), 3));

    let (plain, loud) = plain_and_loud(String::from("Alice"), 2);
    println!("{plain}");
    println!("{loud}");

    println!("{}", call_twice(String::from("ファイト"), 2));
}
