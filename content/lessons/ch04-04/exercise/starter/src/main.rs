fn main() {
    // 標準入力からログファイルのベース名を 1 行読み込み、前後の空白と改行を取り除く（読み込み方は 9-5「標準入力を読む」で詳しく学ぶ）
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let base_name = String::from(line.trim());

    let mut file_name = base_name;
    file_name.push_str(".log");
    println!("{base_name}");
    println!("{file_name}");
}
