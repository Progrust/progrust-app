---
type: Specification
title: "許可クレートと作業領域"
description: "許可クレートの一覧（`runner/allowed-crates.toml`）と同梱の `Cargo.lock`、`workspace/` のディレクトリ構成とビルド先の分け方、アプリ起動時の初回準備の条件・手順・準備中や失敗時の動作、演習のビルドの失敗から初回準備のやり直しを判定する方法と、判定したときの実行・提出の扱いを定める。"
---

## 許可クレート

演習で使える外部クレートはリポジトリの `runner/allowed-crates.toml` に定める。

```toml
[dependencies]
rand = "=0.9.1"
```

- 書式は `Cargo.toml` の `[dependencies]` と同じとし、バージョンは `=` で固定する。
- この一覧から生成した `runner/Cargo.lock` もリポジトリに置き、アプリに同梱する。`Cargo.lock` は、下の「作業領域」の `allcrates/` のライブラリ（パッケージ名 `progrust_allowed_crates`）をルートのパッケージにして生成する。
- クレートを追加・変更したら `Cargo.lock` を作り直す。

## 作業領域

```
{app_data_dir}/workspace/
├── prepared.json         # 初回準備の状態
├── allcrates/            # 全許可クレートに依存する空のライブラリ
├── warmup/               # 初回準備用のプロジェクト
├── project/              # 実行・提出のたびに組み立てるプロジェクト
├── validate/             # 生成問題の検証用プロジェクト（spec/exercise-generation/output-and-validation.md「検証」）
├── target/               # project/ の、許可クレートを使う演習のビルド先（初回準備と共有）
├── target-std/           # project/ の、許可クレートを使わない演習のビルド先
├── target-validate/      # validate/ の、許可クレートを使う問題のビルド先（target/ の複製）
└── target-std-validate/  # validate/ の、許可クレートを使わない問題のビルド先
```

- `allcrates/` は、パッケージ名 `progrust_allowed_crates`、edition 2024 のライブラリで、`[dependencies]` は `allowed-crates.toml` の内容そのまま、`src/lib.rs` は空とする。`warmup/`・`project/`・`validate/` は、どれもこのライブラリに path 依存する（[実行・提出時のプロジェクトの組み立て](project-assembly.md)）。これにより、許可クレートの feature の組み合わせが、どの演習のビルドでも初回準備と同じになる。
- ビルド先は `CARGO_TARGET_DIR` で指定する。許可クレートを使う演習は、初回準備でビルド済みの依存を再利用できる。
- 許可クレートを使わない演習は `target-std/` にビルドし、初回準備の `target/` のロックを待たずにビルドできる。ただし、初回準備がクレートをダウンロードしている間は、cargo のパッケージキャッシュのロックで待つことがある。
- `validate/` は、`project/` と別のビルド先（`target-validate/`・`target-std-validate/`）を使う。生成問題の検証と学習者の実行・提出は、互いのビルドを待たずに並行して行える。
- クレートのダウンロード先（レジストリのキャッシュ）は学習者の環境の既定（`CARGO_HOME`）を使う。

## 初回準備

**実行する条件**（アプリ起動時に判定する）

- `prepared.json` がない
- `prepared.json` の `crates_hash`（`allowed-crates.toml` と `Cargo.lock` のハッシュ値）が同梱のものと異なる
- `prepared.json` の `rustc_version`（`rustc -vV` の出力）が、今の `rustc -vV` の出力と異なる
- `prepared.json` の `status` が `failed`

起動時のほかに、演習のビルドの失敗から初回準備のやり直しが要ると判定したときにも実行する（下の「初回準備のやり直しが要る失敗」）。

**手順**

1. `allcrates/` を作る（上の「作業領域」）。
2. `warmup/` に、パッケージ名 `progrust_warmup`、edition 2024 で、`[dependencies]` が `allcrates/` への path 依存 1 行だけの `Cargo.toml`、同梱の `Cargo.lock`、`fn main() {}` だけを書いた `src/main.rs` を作る。
3. `cargo build` と `cargo test --no-run` を実行する（ネットワークが必要）。ビルド先は `target/`。
4. `target-validate/` を消し、`target/` を `target-validate/` に複製する。複製は APFS のクローン（`cp -c`）で行う。複製した後、`target-validate/` から、学習者の `project/` のビルド結果（パッケージ `progrust_exercise` の実行ファイル、`deps/` の `progrust_exercise-*`、`.fingerprint/` の `progrust_exercise-*`）を消す。
5. 成功したら `prepared.json` に `status: ready` と `crates_hash`・`rustc_version`・`prepared_at` を書く。失敗したら `status: failed` と エラー内容を書き、次回起動時に再度実行する。

- 初回準備のビルドにも、演習のビルドにも、`--locked` と `--frozen` を付けない。同梱の `Cargo.lock` はルートのパッケージが `progrust_allowed_crates` で、演習が使わないクレートの entry も含む。cargo はビルドのときに、これを各プロジェクトに合わせて書き換える。

**準備中・失敗時の動作**

- 許可クレートを使わない演習は通常どおり実行・提出できる。
- 許可クレートを使う演習は、実行・テスト・提出のボタンを無効にし、「クレートを準備しています」「クレートの準備に失敗しました（ネットワーク接続を確認してください）」と表示する。失敗時は再試行ボタンを表示する。
- 許可クレートを使う生成問題の検証は、初回準備が終わるまで始めない。
- 準備の状態は `environment_updated` イベントで通知する。

## 初回準備のやり直しが要る失敗

`--offline` での演習のビルド（`project/`・`validate/`）が終了コード 101 で失敗し、cargo 自身の標準エラー出力（`--message-format=json` の `compiler-message` ではない行）が次のどれかに当てはまるときは、許可クレートのソースか、ビルド済みの依存が失われている。

| 状態 | 判定に使う文 |
|---|---|
| ダウンロード済みの `.crate` が無い | `failed to download` と、`--offline` に触れる文（`attempting to make an HTTP request, but --offline was specified`） |
| レジストリの索引が無い | `no matching package named` と `location searched: crates.io index` |
| ビルド済みの依存が壊れている | `found invalid metadata files for crate`（E0786）で、`note:` が示すファイルが `target/` などのビルド先の中にある |

- 文言は cargo の版で変わりうるので、1 つの語だけで判定せず、表の 2 つ以上の語の組で判定する。
- E0786 は学習者のコードのコンパイルエラーと同じ形で出るので、`note:` のパスがビルド先の中を指すことを必ず確かめる。
- 実行・提出のビルドで当てはまったときは、その実行・提出を学習者のコードの誤りとして表示せず、「クレートを準備し直しています」と表示し、`prepared.json` の `status` を `failed` にして初回準備を始める。ネットワークが無いときは初回準備が失敗し、上の「準備中・失敗時の動作」になる。
  - その実行・提出の結果の状態は `crates_unavailable` とする（[コードの実行（run）](run.md)「結果の状態」、[コードの提出（submit）](submit.md)「判定結果」）。試行ログへの記録は [実行・提出の試行ログへの記録](attempt-log.md) に従う。
  - 初回準備が終わっても、その実行・提出を自動ではやり直さない。学習者が、もう一度ボタンを押す。
- 生成問題の検証のビルドで当てはまったときも、`prepared.json` の `status` を `failed` にして初回準備を始める。検証の扱いは [演習問題生成の出力と検証](../exercise-generation/output-and-validation.md)「初回準備のやり直しが要る失敗になった場合」に従う。
- cargo の自動削除（gc）は、学習者の既定の `CARGO_HOME` で、長く使っていないクレートの `.crate` とソースを消す。そのクレートを使う演習のビルドは、表の 1 行目の失敗になる。
