fn main() {
    // 標準入力から名前を 1 行読み込む（いまはおまじないとして使う。詳しくは第 9 章で学ぶ）
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let name = String::from(line.trim());

    let mut message = name;
    message.push_str("さん、こんにちは");
    println!("{name}");
    println!("{message}");
}
