## 考え方

この問題では、コードを 2 つのモジュールに分け、片方のモジュールからもう片方を使います。

まず `dice` モジュールの `roll` を作ります。`rand::random_range` に範囲を渡すと、その範囲の乱数が 1 つ返ります。サイコロの目は 1 以上 `sides` 以下なので、`1..=sides` を渡します。`rand` は外部クレートなので、`rand::random_range` のように、クレートの名前から始まるパスで呼びます。

次に `game` モジュールの `roll_all` を作ります。空の `Vec` を作り、`for` で `count` 回、`dice::roll(sides)` の結果を `push` します。`dice` は `game` の外にあるモジュールなので、`game` の中の `use crate::dice;` で、ルートから始まる絶対パスで取り込みます。

最後に、`roll` と `roll_all` に `pub` を付けます。モジュールの中の関数は、既定では、そのモジュールの中でしか使えません。`dice::roll` は `game` から、`game::roll_all` は `main` と隠しテストから呼ぶので、どちらも `pub` が必要です。

乱数の値そのものは毎回変わるので、隠しテストは、値の範囲と個数だけを確かめます。

## 模範解答

```rust
mod dice {
    pub fn roll(sides: u32) -> u32 {
        rand::random_range(1..=sides)
    }
}

mod game {
    use crate::dice;

    pub fn roll_all(sides: u32, count: usize) -> Vec<u32> {
        let mut results = Vec::new();
        for _ in 0..count {
            results.push(dice::roll(sides));
        }
        results
    }
}

fn main() {
    let rolls = game::roll_all(6, 5);
    println!("{rolls:?}");
}
```

## ポイント

- **モジュールの定義**: `mod dice { ... }` で、関数をまとめたモジュールを作ります。モジュールの中の関数は、`dice::roll` のように、モジュール名と `::` をつけて呼びます。
- **公開範囲**: `pub` を付けた関数だけが、モジュールの外から呼べます。付けないと、同じモジュールの中でしか使えません。
- **use とパス**: `use crate::dice;` の `crate` は、ファイル全体のルートを指します。`game` モジュールの中で `dice` を使うために、ルートから `dice` までのパスを書いて取り込みます。取り込んだ `dice` は、`dice::roll(sides)` のように使います。
- **外部クレート**: `rand` のような外部クレートの関数は、`rand::random_range(...)` のように、クレートの名前から始まるパスで呼びます。外部クレートを使うには `Cargo.toml` の `[dependencies]` に書く必要があります。この演習では、アプリが書いてくれます。

## よくある間違い

**pub を付け忘れる**

`dice::roll` に `pub` を付けないと、`game` から呼んだところで、非公開の関数を使ったエラー（E0603）になります。

```text
error[E0603]: function `roll` is private
  --> src/main.rs:13:32
   |
13 |             results.push(dice::roll(sides));
   |                                ^^^^ private function
   |
note: the function `roll` is defined here
  --> src/main.rs:2:5
   |
 2 |     fn roll(sides: u32) -> u32 {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^
```

`note` の行が、`roll` を定義した場所を示しています。そこに `pub` を付けます。`game::roll_all` に付け忘れたときは、`main` や隠しテストから呼ぶところで、同じエラーになります。

**use を書かずに dice を使う**

`game` モジュールの中で `use crate::dice;` を書かずに `dice::roll` を呼ぶと、`dice` が見つからないエラー（E0433）になります。モジュールの中では、外にあるモジュールの名前は、そのままでは見えないからです。

```text
error[E0433]: cannot find module or crate `dice` in this scope
  --> src/main.rs:11:26
   |
11 |             results.push(dice::roll(sides));
   |                          ^^^^ use of unresolved module or unlinked crate `dice`
   |
   = help: you might be missing a crate named `dice`
help: consider importing this module
   |
 8 +     use crate::dice;
   |
```

`help` に、足すべき `use` の行が出ています。

**モジュール名を付けずに roll を呼ぶ**

`use crate::dice;` で取り込んだのは `dice` というモジュールの名前だけです。`roll(sides)` と書くと、関数が見つからないエラー（E0425）になります。`dice::roll(sides)` と書きます。

```text
error[E0425]: cannot find function `roll` in this scope
  --> src/main.rs:13:26
   |
13 |             results.push(roll(sides));
   |                          ^^^^ not found in this scope
   |
help: consider importing this function
   |
 8 +     use crate::dice::roll;
   |
```

**use のパスを間違える**

`use crate::dices;` のように、存在しない名前を書くと、取り込めないエラー（E0432）になります。

```text
error[E0432]: unresolved import `crate::dices`
 --> src/main.rs:8:9
  |
8 |     use crate::dices;
  |         ^^^^^^^-----
  |                |
  |                no `dices` in the root
  |
help: a similar name exists in the module
  |
8 -     use crate::dices;
8 +     use crate::dice;
  |
```

`no `dices` in the root` は、ルートに `dices` という名前が無いという意味です。`help` が、似た名前の `dice` を示しています。
