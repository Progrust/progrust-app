fn go_up(path: String, levels: u32) -> String {
    let mut result = path;
    let mut added = 0;
    while added < levels {
        result.push_str("/..");
        added = added + 1;
    }
    result
}

fn path_and_up(path: String, levels: u32) -> (String, String) {
    let upper = go_up(path.clone(), levels);
    (path, upper)
}

fn diff_args(path: String, levels: u32) -> String {
    let before = path.clone();
    let after = go_up(path, levels);
    format!("{before} {after}")
}

fn main() {
    println!("{}", go_up(String::from("logs/app"), 2));

    let (current, upper) = path_and_up(String::from("logs/app"), 1);
    println!("{current}");
    println!("{upper}");

    println!("{}", diff_args(String::from("config"), 1));
}
