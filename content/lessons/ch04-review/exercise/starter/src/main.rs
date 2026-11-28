fn go_up(path: String, levels: u32) -> String {
    // ここを書く（いまは仮の値を返している）
    path
}

fn path_and_up(path: String, levels: u32) -> (String, String) {
    // ここを書く（いまは仮の値を返している）
    (path, String::from(""))
}

fn diff_args(path: String, levels: u32) -> String {
    // ここを書く（いまは仮の値を返している）
    path
}

fn main() {
    println!("{}", go_up(String::from("logs/app"), 2));

    let (current, upper) = path_and_up(String::from("logs/app"), 1);
    println!("{current}");
    println!("{upper}");

    println!("{}", diff_args(String::from("config"), 1));
}
