## 考え方

3 つの関数は、どれも `String` を引数で受け取ります。`String` を関数に渡すと、所有権は関数の引数に移ります（ムーブ）。そのため、関数に渡した後の変数は、もう使えません。

まず `go_up` を作ります。受け取った `path` の所有権を `let mut result = path;` で変更できる変数に移し、足した個数を数えながら、`while` で `levels` 回 `push_str` して「/..」を足します。最後に `result` を戻り値として返すと、所有権は呼び出し元に移ります。

`path_and_up` では、`path` をそのまま返すことと、`path` から「/..」を付けた文字列を作ることの両方が要ります。`go_up(path, levels)` と渡すと、`path` の所有権が `go_up` に移り、後で `path` を返せません。`path.clone()` でヒープの文字列ごと複製した `String` を `go_up` に渡し、元の `path` は手元に残して返します。

`diff_args` では、元のパスと、「/..」を付けたパスの両方を使います。元のパスの分は `path.clone()` で複製して `before` に取っておき、`go_up` には `path` そのものを渡します。`go_up` に渡した後は `path` を使わないので、ここで所有権を渡してかまいません。`levels` は `u32` で、`Copy` な型なので、関数に渡してもムーブされず、そのまま使えます。

## 模範解答

```rust
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
```

## ポイント

- **String 型**: `String` はヒープに置かれる文字列で、`push_str` で後ろに文字列を足せます。`format!` は、埋め込んだ値から新しい `String` を作ります。文字列リテラル（`"logs/app"`）は `String` ではないので、`String` を受け取る関数には `String::from` で作った値を渡します。
- **所有権のムーブ**: `let mut result = path;` の代入で、`path` が持っていた `String` の所有権は `result` に移ります。ヒープの文字列は複製されず、所有者だけが変わります。
- **関数と所有権**: 引数に渡した `String` の所有権は関数に移り、戻り値で返した `String` の所有権は呼び出し元に移ります。`go_up` は、受け取った `String` に「/..」を足してから返すので、余分な複製をしません。
- **clone と Copy**: 渡した後も使いたい `String` は、`clone` で複製したものを渡します。`levels` のような整数は `Copy` な型なので、関数に渡してもコピーされるだけで、元の変数をそのまま使い続けられます。
- **実務での使われ方**: パスやファイル名、メッセージのような文字列に手を加えて別の文字列を作る関数は、実務のコードによく出てきます。受け取った `String` に手を加えて返す書き方は、複製が要らないので無駄がありません。元の値も残したいときだけ `clone` を使います。ただし `clone` は、ヒープの文字列ごと複製するので、必要な箇所だけに使います。複製せずに元の値を見せるだけで済む場面では、5 章で学ぶ参照を使う書き方が、実務ではよく使われます。

## よくある間違い

**関数に渡した後の path を使う**

`path_and_up` で、`path` を複製せずに `go_up` に渡すと、ムーブ済みの値を使ったエラー（E0382）になります。

```text
error[E0382]: use of moved value: `path`
  --> src/main.rs:13:6
   |
11 | fn path_and_up(path: String, levels: u32) -> (String, String) {
   |                ---- move occurs because `path` has type `String`, which does not implement the `Copy` trait
12 |     let upper = go_up(path, levels);
   |                       ---- value moved here
13 |     (path, upper)
   |      ^^^^ value used here after move
```

`value moved here` の行（12 行目）で、`path` の所有権が `go_up` に移っています。`diff_args` で、`path` を `go_up` に渡した後に、もう一度 `path` を使ったときも、同じ E0382 になります。後でもう一度使う `String` は、`path.clone()` を渡します。

一方、`levels` は 2 回渡してもエラーになりません。`u32` は `Copy` な型で、関数に渡すときにコピーされるからです。

**result を `mut` にしない**

`go_up` で `let result = path;` のように `mut` を付けないと、`push_str` で文字列を変えられず、変更できない変数を変えようとしたエラー（E0596）になります。

```text
error[E0596]: cannot borrow `result` as mutable, as it is not declared as mutable
 --> src/main.rs:5:9
  |
5 |         result.push_str("/..");
  |         ^^^^^^ cannot borrow as mutable
```

ムーブで所有者が変わっても、変更できるかどうかは、受け取る側の変数の宣言（`let mut`）で決まります。

**文字列リテラルをそのまま渡す**

`main` で `go_up("logs/app", 2)` のように文字列リテラルを渡すと、型が合わないエラー（E0308）になります。

```text
error[E0308]: mismatched types
  --> src/main.rs:23:26
   |
23 |     println!("{}", go_up("logs/app", 2));
   |                    ----- ^^^^^^^^^^ expected `String`, found `&str`
   |                    |
   |                    arguments to this function are incorrect
```

`go_up` の引数は `String` なので、`String::from("logs/app")` で `String` を作って渡します。
