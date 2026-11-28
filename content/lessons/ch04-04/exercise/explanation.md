## 考え方

初期コードの 7 行目 `let mut file_name = base_name;` で、base_name が持っていた String の所有権は file_name に移ります（ムーブ）。ムーブした後の base_name は使えないので、9 行目の `println!("{base_name}");` がコンパイルエラーになります。

base_name を使うのは、所有権を file_name に渡す前にします。1 行目のベース名を出力してから、base_name を file_name にムーブし、file_name に「.log」を足して出力します。

## 模範解答

```rust
fn main() {
    // 標準入力からログファイルのベース名を 1 行読み込み、前後の空白と改行を取り除く（読み込み方は 9-5「標準入力を読む」で詳しく学ぶ）
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let base_name = String::from(line.trim());

    println!("{base_name}");
    let mut file_name = base_name;
    file_name.push_str(".log");
    println!("{file_name}");
}
```

## ポイント

- `let mut file_name = base_name;` のように String を代入すると、所有権が base_name から file_name に移ります。ヒープ上の文字列は複製されず、所有者だけが変わります。
- ムーブした後は、元の変数 base_name を使えません。値を使えるのは、いまの所有者（file_name）だけです。
- そのため、元の変数で済ませたい処理（ここではベース名の出力）は、ムーブより前に書きます。
- 実務では、ファイル名やメッセージなど、元の文字列に手を加えて別の文字列を作る場面がよくあります。元の値をログに出す、検査に使う、といった処理が要るときは、手を加える前に済ませるのが、複製せずに書く基本です。元の値を後でも使いたいときの書き方は、4-5 で複製（clone）を、5 章で参照を学びます。

## よくある間違い

**ムーブした後に base_name を使う**

初期コードのように、`let mut file_name = base_name;` の後で base_name を使うと、ムーブ済みの値を使ったエラー（E0382）になります。

```text
error[E0382]: borrow of moved value: `base_name`
 --> src/main.rs:9:16
  |
5 |     let base_name = String::from(line.trim());
  |         --------- move occurs because `base_name` has type `String`, which does not implement the `Copy` trait
6 |
7 |     let mut file_name = base_name;
  |                         --------- value moved here
8 |     file_name.push_str(".log");
9 |     println!("{base_name}");
  |                ^^^^^^^^^ value borrowed here after move
```

`value moved here` の行が、所有権が移った場所（7 行目）を指しています。base_name を使う処理を、この行より前に移します。

**file_name を `mut` にしない**

`let file_name = base_name;` のように `mut` を付けないと、`push_str` で文字列を変えられず、変更できない変数を変えようとしたエラー（E0596）になります。

```text
error[E0596]: cannot borrow `file_name` as mutable, as it is not declared as mutable
 --> src/main.rs:9:5
  |
9 |     file_name.push_str(".log");
  |     ^^^^^^^^^ cannot borrow as mutable
```

ムーブで所有者が変わっても、変更できるかどうかは、受け取る側の変数の宣言（`let mut`）で決まります。
