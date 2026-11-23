## 考え方

初期コードの 7 行目 `let mut message = name;` で、name が持っていた String の所有権は message に移ります（ムーブ）。ムーブした後の name は使えないので、9 行目の `println!("{name}");` がコンパイルエラーになります。

name を使うのは、所有権を message に渡す前にします。1 行目の名前を出力してから、name を message にムーブし、message に「さん、こんにちは」を足して出力します。

## 模範解答

```rust
fn main() {
    // 標準入力から名前を 1 行読み込む（いまはおまじないとして使う。詳しくは第 9 章で学ぶ）
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let name = String::from(line.trim());

    println!("{name}");
    let mut message = name;
    message.push_str("さん、こんにちは");
    println!("{message}");
}
```

## ポイント

- `let mut message = name;` のように String を代入すると、所有権が name から message に移ります。ヒープ上の文字列は複製されず、所有者だけが変わります。
- ムーブした後は、元の変数 name を使えません。値を使えるのは、いまの所有者（message）だけです。
- そのため、元の変数で済ませたい処理（ここでは名前の出力）は、ムーブより前に書きます。

## よくある間違い

**ムーブした後に name を使う**

初期コードのように、`let mut message = name;` の後で name を使うと、ムーブ済みの値を使ったエラー（E0382）になります。

```text
error[E0382]: borrow of moved value: `name`
 --> src/main.rs:9:16
  |
5 |     let name = String::from(line.trim());
  |         ---- move occurs because `name` has type `String`, which does not implement the `Copy` trait
6 |
7 |     let mut message = name;
  |                       ---- value moved here
8 |     message.push_str("さん、こんにちは");
9 |     println!("{name}");
  |                ^^^^ value borrowed here after move
```

`value moved here` の行が、所有権が移った場所（7 行目）を指しています。name を使う処理を、この行より前に移します。

**message を `mut` にしない**

`let message = name;` のように `mut` を付けないと、`push_str` で文字列を変えられず、変更できない変数を変えようとしたエラー（E0596）になります。

```text
error[E0596]: cannot borrow `message` as mutable, as it is not declared as mutable
 --> src/main.rs:9:5
  |
9 |     message.push_str("さん、こんにちは");
  |     ^^^^^^^ cannot borrow as mutable
```

ムーブで所有者が変わっても、変更できるかどうかは、受け取る側の変数の宣言（`let mut`）で決まります。
