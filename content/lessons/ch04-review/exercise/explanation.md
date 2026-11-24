## 考え方

3 つの関数は、どれも `String` を引数で受け取ります。`String` を関数に渡すと、所有権は関数の引数に移ります（ムーブ）。そのため、関数に渡した後の変数は、もう使えません。

まず `exclaim` を作ります。受け取った `text` の所有権を `let mut result = text;` で変更できる変数に移し、足した個数を数えながら、`while` で `count` 回 `push_str` して「!」を足します。最後に `result` を戻り値として返すと、所有権は呼び出し元に移ります。

`plain_and_loud` では、`text` をそのまま返すことと、`text` から「!」を付けた文字列を作ることの両方が要ります。`exclaim(text, count)` と渡すと、`text` の所有権が `exclaim` に移り、後で `text` を返せません。`text.clone()` でヒープの文字列ごと複製した `String` を `exclaim` に渡し、元の `text` は手元に残して返します。

`call_twice` では、同じ文字列から「!」を付けた文字列を 2 つ作ります。1 回目の呼び出しには `text.clone()` を渡し、2 回目には `text` そのものを渡します。2 回目の後は `text` を使わないので、ここで所有権を渡してかまいません。`count` は `u32` で、`Copy` な型なので、関数に渡してもムーブされず、2 回目にもそのまま使えます。

## 模範解答

```rust
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
```

## ポイント

- **String 型**: `String` はヒープに置かれる文字列で、`push_str` で後ろに文字列を足せます。`format!` は、埋め込んだ値から新しい `String` を作ります。文字列リテラル（`"がんばれ"`）は `String` ではないので、`String` を受け取る関数には `String::from` で作った値を渡します。
- **所有権のムーブ**: `let mut result = text;` の代入で、`text` が持っていた `String` の所有権は `result` に移ります。ヒープの文字列は複製されず、所有者だけが変わります。
- **関数と所有権**: 引数に渡した `String` の所有権は関数に移り、戻り値で返した `String` の所有権は呼び出し元に移ります。`exclaim` は、受け取った `String` に「!」を足してから返すので、余分な複製をしません。
- **clone と Copy**: 渡した後も使いたい `String` は、`clone` で複製したものを渡します。`count` のような整数は `Copy` な型なので、関数に渡してもコピーされるだけで、元の変数をそのまま使い続けられます。

## よくある間違い

**関数に渡した後の text を使う**

`plain_and_loud` で、`text` を複製せずに `exclaim` に渡すと、ムーブ済みの値を使ったエラー（E0382）になります。

```text
error[E0382]: use of moved value: `text`
  --> src/main.rs:13:6
   |
11 | fn plain_and_loud(text: String, count: u32) -> (String, String) {
   |                   ---- move occurs because `text` has type `String`, which does not implement the `Copy` trait
12 |     let loud = exclaim(text, count);
   |                        ---- value moved here
13 |     (text, loud)
   |      ^^^^ value used here after move
```

`value moved here` の行（12 行目）で、`text` の所有権が `exclaim` に移っています。`call_twice` で、2 回とも `exclaim(text, count)` と書いたときも、同じ E0382 になり、2 回目の呼び出しの `text` が指されます。後でもう一度使う `String` は、`text.clone()` を渡します。

一方、`count` は 2 回渡してもエラーになりません。`u32` は `Copy` な型で、関数に渡すときにコピーされるからです。

**result を `mut` にしない**

`exclaim` で `let result = text;` のように `mut` を付けないと、`push_str` で文字列を変えられず、変更できない変数を変えようとしたエラー（E0596）になります。

```text
error[E0596]: cannot borrow `result` as mutable, as it is not declared as mutable
 --> src/main.rs:5:9
  |
5 |         result.push_str("!");
  |         ^^^^^^ cannot borrow as mutable
```

ムーブで所有者が変わっても、変更できるかどうかは、受け取る側の変数の宣言（`let mut`）で決まります。

**文字列リテラルをそのまま渡す**

`main` で `exclaim("がんばれ", 3)` のように文字列リテラルを渡すと、型が合わないエラー（E0308）になります。

```text
error[E0308]: mismatched types
  --> src/main.rs:23:28
   |
23 |     println!("{}", exclaim("がんばれ", 3));
   |                    ------- ^^^^^^^^^^ expected `String`, found `&str`
   |                    |
   |                    arguments to this function are incorrect
```

`exclaim` の引数は `String` なので、`String::from("がんばれ")` で `String` を作って渡します。
