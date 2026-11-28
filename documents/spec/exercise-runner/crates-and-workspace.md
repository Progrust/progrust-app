---
type: Specification
title: "許可クレートと作業領域"
description: "許可クレートの一覧（`runner/allowed-crates.toml`）と同梱の `Cargo.lock`、`workspace/` のディレクトリ構成（C の演習の `c-project/` を含む）とビルド先の分け方、アプリ起動時と再検出のときの初回準備の条件・手順・準備中や失敗時の動作・アプリの終了のときの扱い、`prepared.json` の形、バックエンドが持つ初回準備の状態（`not_started`・`preparing`・`ready`・`failed`）、演習のビルドの失敗から初回準備のやり直しを判定する方法と、判定したときの実行・提出の扱いを定める。"
---

許可クレートと初回準備は、Rust の演習（レッスンの演習のうち Rust のレッスンのものと、生成問題）のものである。C の演習は許可クレートを使わず、初回準備と関係しない（[C の演習の作業領域とビルド](c-project-assembly.md)「作業領域」）。

## 許可クレート

演習で使える外部クレートはリポジトリの `runner/allowed-crates.toml` に定める。

```toml
[dependencies]
rand = "=0.9.5"
```

- 書式は `Cargo.toml` の `[dependencies]` と同じとし、バージョンは `=` で固定する。
- この一覧から生成した `runner/Cargo.lock` もリポジトリに置き、アプリに同梱する。`Cargo.lock` は、下の「作業領域」の `allcrates/` のライブラリ（パッケージ名 `progrust_allowed_crates`）をルートのパッケージにして生成する。
- クレートを追加・変更したら `Cargo.lock` を作り直す。`npm run runner:lock` が、一時ディレクトリに `allcrates/` と同じ形のライブラリを作り、`cargo generate-lockfile` で `runner/Cargo.lock` を作り直す。

## 作業領域

```
{app_data_dir}/workspace/
├── prepared.json         # 初回準備の状態
├── allcrates/            # 全許可クレートに依存する空のライブラリ
├── warmup/               # 初回準備用のプロジェクト
├── project/              # Rust の演習の実行・提出のたびに組み立てるプロジェクト
├── c-project/            # C の演習の実行・提出のたびに組み立てる作業領域（spec/exercise-runner/c-project-assembly.md）
├── validate/             # 生成問題の検証用プロジェクト（spec/exercise-generation/output-and-validation.md「検証」）
├── target/               # project/ の、許可クレートを使う演習のビルド先（初回準備と共有）
├── target-std/           # project/ の、許可クレートを使わない演習のビルド先
├── target-validate/      # validate/ の、許可クレートを使う問題のビルド先（target/ の複製）
└── target-std-validate/  # validate/ の、許可クレートを使わない問題のビルド先
```

- `allcrates/` は、パッケージ名 `progrust_allowed_crates`、edition 2024 のライブラリで、`[dependencies]` は `allowed-crates.toml` の内容そのまま、`src/lib.rs` は空とする。`warmup/` と、許可クレートを使う演習の `project/`・`validate/` は、このライブラリに path 依存する。許可クレートを使わない演習の `project/`・`validate/` は、path 依存しない（[実行・提出時のプロジェクトの組み立て](project-assembly.md)）。これにより、許可クレートの feature の組み合わせが、許可クレートを使うどの演習のビルドでも初回準備と同じになり、許可クレートを使わない演習のビルドは、許可クレートのソースもビルド結果も要らない。
- ビルド先は `CARGO_TARGET_DIR` で指定する。許可クレートを使う演習は、初回準備でビルド済みの依存を再利用できる。
- 許可クレートを使わない演習は `target-std/` にビルドし、初回準備の `target/` のロックを待たずにビルドできる。ただし、初回準備がクレートをダウンロードしている間は、cargo のパッケージキャッシュのロックで待つことがある。
- `validate/` は、`project/` と別のビルド先（`target-validate/`・`target-std-validate/`）を使う。生成問題の検証と学習者の実行・提出は、互いのビルドを待たずに並行して行える。
- クレートのダウンロード先（レジストリのキャッシュ）は学習者の環境の既定（`CARGO_HOME`）を使う。
- `c-project/` は、C の演習だけが使う。`cargo` を使わず、ほかのディレクトリとビルド先に関係しない。C の演習の実行・提出は、初回準備の状態（下の「初回準備の状態」）にも、`cargo` が見つかるかどうかにも左右されない。

## 初回準備

**実行する条件**（アプリ起動時に判定する。`cargo` が見つからないときは判定しない）

- `prepared.json` がない
- `prepared.json` の `crates_hash`（`allowed-crates.toml` と `Cargo.lock` のハッシュ値）が同梱のものと異なる
- `prepared.json` の `rustc_version`（`rustc -vV` の出力）が、今の `rustc -vV` の出力と異なる
- `prepared.json` の `status` が `failed`

- `crates_hash` は、同梱の `allowed-crates.toml` のバイト列に `Cargo.lock` のバイト列を続けたものの SHA-256 を、小文字の 16 進数で書いたものとする。

起動時のほかに、次のときにも実行する。

- 演習のビルドの失敗から、初回準備のやり直しが要ると判定したとき（下の「初回準備のやり直しが要る失敗」）。
- 「再検出」（[環境のコマンドとイベント](../architecture/commands/environment.md)「`redetect_tools`」）で `cargo` が見つかり、初回準備の状態が `preparing` でなく、「実行する条件」のどれかに当てはまるとき。起動時に `cargo` が見つからなかった後に見つかったときと、設定のパスを変えて別の `rustc` になったときが、これに当たる。
  - どれにも当てはまらないときは、初回準備の状態を `ready` にする（起動時に `cargo` が見つからず `not_started` だった後に見つかり、前に準備したものがそのまま使えるとき）。
  - 初回準備の状態が `preparing` のときは、動いている初回準備を止めず、条件も判定し直さない。
  - 再検出で `cargo` が見つからなくなったときは、初回準備の状態を変えない。

**手順**

1. `allcrates/` を作る（上の「作業領域」）。
2. `warmup/` に、パッケージ名 `progrust_warmup`、edition 2024 で、`[dependencies]` が `allcrates/` への path 依存 1 行だけの `Cargo.toml`、同梱の `Cargo.lock`、`fn main() {}` だけを書いた `src/main.rs` を作る。
3. `cargo build` と `cargo test --no-run` を実行する（ネットワークが必要）。ビルド先は `target/`。
4. `target-validate/` を消し、`target/` を `target-validate/` に複製する。複製は APFS のクローン（`cp -c`）で行う。複製した後、`target-validate/` から、学習者の `project/` のビルド結果（パッケージ `progrust_exercise` の実行ファイルと、そのビルドの途中の成果物と fingerprint）を消す。途中の成果物と fingerprint の置き場所は cargo の版で変わり、cargo 1.100.0 では `build/progrust_exercise/`、cargo 1.99.1 では `deps/` と `.fingerprint/` の `progrust_exercise-*` である。どちらの置き場所のものも消す。
5. 成功したら `prepared.json` に `status: ready` と `crates_hash`・`rustc_version`・`prepared_at` を書く。失敗したら `status: failed` と エラー内容を書き、次回起動時に再度実行する。

- アプリを終了するときに初回準備が動いていたら、初回準備の `cargo` を止め、`prepared.json` は書かない。次回起動時には「実行する条件」に当てはまり、初回準備をやり直す（初回準備を始めるときの `prepared.json` は、無いか、`status` が `failed` か、`crates_hash`・`rustc_version` が今のものと異なる）。

- 初回準備のビルドにも、演習のビルドにも、`--locked` と `--frozen` を付けない。同梱の `Cargo.lock` はルートのパッケージが `progrust_allowed_crates` で、演習が使わないクレートの entry も含む。cargo はビルドのときに、これを各プロジェクトに合わせて書き換える。

**`prepared.json`**

```json
{
  "schema_version": 1,
  "status": "ready",
  "crates_hash": "3f5a…",
  "rustc_version": "rustc 1.100.0 (…)\nbinary: rustc\n…",
  "prepared_at": "2026-11-24T19:02:00+09:00",
  "error": null
}
```

| 項目 | 内容 |
|---|---|
| `schema_version` | `1` |
| `status` | `ready` か `failed` |
| `crates_hash` | 初回準備に使った `allowed-crates.toml` と `Cargo.lock` のハッシュ値（上の「実行する条件」） |
| `rustc_version` | 初回準備に使った `rustc` の `rustc -vV` の出力そのまま |
| `prepared_at` | 初回準備が終わった日時（[コマンドとイベントの共通の決まり](../architecture/commands/conventions.md) の日時の書き方） |
| `error` | `failed` のときのエラーの内容。`ready` のときは `null` |

- 書き込みは [データの保存場所](../architecture/data-locations.md)「書き込みの規則」の一時ファイルとリネームで行う。
- 読み込めない `prepared.json`（JSON として読めない、この形に合わない、`schema_version` が `1` でない）は、無いものとして扱う。退避も通知もしない。初回準備をやり直せば作り直せる、学習者のデータではないファイルだからである。

**準備中・失敗時の動作**

- 許可クレートを使わない演習と、C の演習は、通常どおり実行・提出できる。
- 許可クレートを使う演習は、実行・テスト・提出のボタンを無効にし、「外部クレートを準備しています」「外部クレートの準備に失敗しました（ネットワーク接続を確認してください）」と表示する。失敗時は再試行ボタンを表示する。
- 許可クレートを使う生成問題の検証は、初回準備が終わるまで始めない。
- 準備の状態は `environment_updated` イベントで通知する（[環境のコマンドとイベント](../architecture/commands/environment.md)）。

**初回準備の状態**

バックエンドは、いまの初回準備の状態を、次の 4 つの値でメモリに持つ。`prepared.json` の `status`（`ready` / `failed`）は、最後に終わった初回準備の結果で、準備中を表す値を持たない。

| 状態 | 意味 |
|---|---|
| `not_started` | 初回準備を始めていない。`cargo` が見つからず、「実行する条件」を判定していないときを含む |
| `preparing` | 初回準備を実行している |
| `ready` | 初回準備が成功して終わり、「実行する条件」のどれにも当てはまらない |
| `failed` | 最後の初回準備が失敗した |

- 「準備中・失敗時の動作」の「準備中」は `preparing`、「失敗時」は `failed` のときを指す。

## 初回準備のやり直しが要る失敗

`--offline` での演習のビルド（`project/`・`validate/`）が終了コード 101 で失敗し、ビルドの出力が次のどれかに当てはまるときは、許可クレートのソースか、作業領域の `allcrates/` か、ビルド済みの依存が失われている。ビルドの出力は、cargo 自身の標準エラー出力と、コンパイラの診断（cargo の標準出力の JSON のうち `compiler-message` の診断）に分かれて出る（[コードの実行（run）](run.md)「ビルドの出力の表示」）。判定に使う文は、表の「探す出力」の列の出力から探す。

| 状態 | 探す出力 | 判定に使う文 |
|---|---|---|
| ダウンロード済みの `.crate` が無い | cargo 自身の標準エラー出力 | `failed to download` と、`--offline` に触れる文（`attempting to make an HTTP request, but --offline was specified`） |
| レジストリの索引が無い | cargo 自身の標準エラー出力 | `no matching package named` と `location searched: crates.io index` |
| 作業領域の `allcrates/` が無い | cargo 自身の標準エラー出力 | ``no matching package named `progrust_allowed_crates` `` と、`location searched:` の後に `allcrates` のパスが続く行 |
| ビルド済みの依存が壊れている | コンパイラの診断 | `found invalid metadata files for crate`（E0786）の診断で、その `note:` が示すファイルが `target/` などのビルド先の中にある |

- 文言は cargo の版で変わりうるので、1 つの語だけで判定せず、表の 2 つ以上の語の組で判定する。
- E0786 は学習者のコードのコンパイルエラーと同じ形（コンパイラの診断）で出るので、`note:` のパスがビルド先の中を指すことを必ず確かめる。ビルド先の中のどこを指すかは cargo の版で変わりうるので、ビルド先の中であることだけを確かめる。
- 実行・提出のビルドで当てはまったときは、その実行・提出を学習者のコードの誤りとして表示せず、「外部クレートを準備し直しています」と表示し、`prepared.json` の `status` を `failed` にして初回準備を始める。ネットワークが無いときは初回準備が失敗し、上の「準備中・失敗時の動作」になる。
  - その実行・提出の結果の状態は `crates_unavailable` とする（[コードの実行（run）](run.md)「結果の状態」、[コードの提出（submit）](submit.md)「判定結果」）。試行ログへの記録は [実行・提出の試行ログへの記録](attempt-log.md) に従う。
  - 初回準備が終わっても、その実行・提出を自動ではやり直さない。学習者が、もう一度ボタンを押す。
- 生成問題の検証のビルドで当てはまったときも、`prepared.json` の `status` を `failed` にして初回準備を始める。検証の扱いは [演習問題生成の出力と検証](../exercise-generation/output-and-validation.md)「初回準備のやり直しが要る失敗になった場合」に従う。
- cargo の自動削除（gc）は、学習者の既定の `CARGO_HOME` で、長く使っていないクレートの `.crate` とソースを消す。そのクレートを使う演習のビルドは、表の 1 行目の失敗になる。
