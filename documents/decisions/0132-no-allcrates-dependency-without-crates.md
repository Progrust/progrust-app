---
type: Decision
title: "ADR 0132"
description: "許可クレートを使わない演習のプロジェクトには、全許可クレートに依存する空のライブラリへの path 依存を書かない"
---

- 日付: 2026-11-22
- 関連仕様: [実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「作業領域」
- 置き換え: なし

## 背景

[ADR 0097](0097-share-crate-builds-via-allcrates-library.md) は、許可クレートの feature の組み合わせを初回準備とそろえるために、全許可クレートに依存する空のライブラリ `allcrates/` への path 依存を、`warmup/`・`project/`・`validate/` のどれにも書くとした。仕様は、許可クレートを使わない演習でもこの path 依存を書くとしていた。

一方で、許可クレートを使わない演習は、`target-std/` にビルドし、初回準備の終わりを待たずに実行・提出できるとしている（[ADR 0012](0012-separate-build-dir-without-crates.md)、[ADR 0099](0099-allow-package-cache-wait-during-preparation.md)）。

path 依存したライブラリの依存も、ビルドの対象になる。Issue #52 で、そのために ADR 0012・0099 の前提が成り立たないおそれがあると起票された。仕分けのときに、cargo 1.100.0・rustc 1.100.0（macOS 27.2.1）で、次のとおり確かめた。`allcrates/` は `rand = "=0.9.1"` だけに依存する空のライブラリにし、`allcrates/` で生成した `Cargo.lock` を演習のプロジェクトにコピーし、`--offline` でビルドした。

| 演習のプロジェクト | クレートの状態 | 結果 |
|---|---|---|
| path 依存あり | ダウンロード前（空の `CARGO_HOME`） | 終了コード 101。`` error: no matching package named `rand` found ``、`location searched: crates.io index` |
| path 依存あり | 索引はあるが `.crate` が無い | 終了コード 101。`` error: failed to download `zerocopy v0.8.64` ``、`attempting to make an HTTP request, but --offline was specified` |
| path 依存あり | ダウンロード済み | 成功。rand とその依存の 8 クレートを `target-std/` にビルドした（2.76 秒、`target-std/` は 44 MB） |
| path 依存なし | ダウンロード前（空の `CARGO_HOME`） | 成功（0.23 秒、`target-std/` は 1.1 MB） |

- path 依存があると、許可クレートを使わない演習も、クレートのダウンロードが終わるまではビルドに失敗する。失敗の文は「初回準備のやり直しが要る失敗」（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)）の表の 1 行目・2 行目に当てはまる。
- ダウンロードの後も、全許可クレートを `target/` とは別に `target-std/` へビルドする。
- path 依存が無ければ、ダウンロードの前でもビルドできる。[検証と学習者の実行の並行ビルドの技術検証](../verification/issue-13-parallel-build-lock.md)でも、同梱の `Cargo.lock` をコピーした、path 依存の無い std だけの演習を、初回準備の最中に `--offline` でビルドできた。

## 決定

- 許可クレートを使わない演習の `project/`・`validate/` の `Cargo.toml` には、`allcrates/` への path 依存を書かない（[実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「作業領域」）。ADR 0097 の「初回準備の `warmup/` と、実行・提出の `project/`、生成問題の検証の `validate/` が、どれもこれに path 依存する」を、「`warmup/` と、許可クレートを使う演習の `project/`・`validate/` が path 依存する」に変える。
  - 理由: path 依存を書くと、許可クレートを使わない演習が、初回準備のダウンロードが終わるまで実行・提出できず、ADR 0012・0099 の「初回準備の終わりを待たずに実行できる」が成り立たない。ADR 0097 が path 依存を書いたのは、許可クレートの feature の組み合わせを初回準備とそろえるためで、許可クレートを使わない演習には、そろえるクレートが無い。
- 許可クレートを使わない演習にも、同梱の `Cargo.lock` はコピーする。
  - 理由: path 依存の無いプロジェクトに同梱の `Cargo.lock` をコピーしても、`--offline` でビルドできた。演習によってコピーするかどうかを変えずに済む。

## 検討した他の案

- 案: path 依存を書いたまま、許可クレートを使わない演習も、初回準備が終わるまでボタンを無効にする
  - 採用しなかった理由: 初回準備の最中に最初のレッスンの演習を実行できなくなる。ADR 0012 がビルド先を分けた目的が無くなる
- 案: path 依存を書いたまま、許可クレートを使わない演習のビルドの失敗を「初回準備のやり直しが要る失敗」として扱う
  - 採用しなかった理由: 初回準備の最中は、許可クレートを使わない演習が毎回失敗する。ダウンロードの後も、全許可クレートを `target-std/` にもビルドするので、時間とディスクが増える

## 影響

- 許可クレートを使わない演習は、クレートのダウンロードの前でも、ダウンロード済みのクレートが cargo の自動削除で消えた後でも、ビルドできる。ビルドが「初回準備のやり直しが要る失敗」の表の 1 行目・2 行目になることは無い。
- `target-std/`・`target-std-validate/` に許可クレートをビルドしないので、ディスクが増えない。
- 演習を組み立てるときに、許可クレートを使うかどうかで `Cargo.toml` の書き方を分ける必要がある。
