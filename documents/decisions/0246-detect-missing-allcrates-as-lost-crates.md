---
type: Decision
title: "ADR 0246"
description: "作業領域の allcrates/ が無いときの --offline のビルドの失敗（no matching package named の progrust_allowed_crates と、allcrates のパスの location searched）も、初回準備のやり直しが要る失敗として扱う"
---

- 日付: 2026-11-25
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備のやり直しが要る失敗」
- 置き換え: なし

## 背景

作業領域の `prepared.json` が `ready` で、`crates_hash`・`rustc_version` も今のものと同じまま、`allcrates/` だけが無いと、許可クレートを使う演習の `--offline` のビルドは、`no matching package named \`progrust_allowed_crates\` found` と、`location searched:` に `allcrates/` のパスが続く出力で失敗した。この出力は、[ADR 0105](0105-detect-lost-crates-from-offline-build-errors.md) の 3 つの状態のどれにも当てはまらなかった。そのため、実行・提出の結果は `crates_unavailable` ではなく `compile_error` になり、学習者のコードの誤りとして表示され、初回準備もやり直さなかった。「実行する条件」にも当てはまらないので、アプリを起動し直しても直らなかった（Issue #256）。

作業領域のフォルダを学習者が片づけたときなどに起こりうる。

Claude の技術的な判断である。操作感は変わらない（学習者のコードの誤りと表示されていた失敗が、「外部クレートを準備し直しています」の表示になる）。

## 決定

- `allcrates/` が無い状態を、初回準備のやり直しが要る失敗の 4 つ目の状態として、表に足す（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備のやり直しが要る失敗」）。判定は、cargo 自身の標準エラー出力から、`no matching package named \`progrust_allowed_crates\`` と、`location searched:` の後に `allcrates` のパスが続く行の組で行う。
  - 理由: [ADR 0105](0105-detect-lost-crates-from-offline-build-errors.md) の 2 つ以上の語の組で判定する方針と、[ADR 0137](0137-find-e0786-in-compiler-diagnostics.md) の、文が出る出力だけから探す方針に合わせる。`location searched:` の後ろを `allcrates` に限るのは、レジストリの索引の欠落（`crates.io index`）や、ほかのパッケージの欠落と取り違えないためである。
- 判定したときの実行・提出・検証の扱いは、既存の 3 つの状態と同じにする（[ADR 0125](0125-crates-unavailable-status-without-attempt-log.md)、[ADR 0126](0126-redo-validation-after-crates-are-prepared-again.md)）。教材チェックツールも同じにする（[ADR 0245](0245-content-check-prepares-again-on-lost-crates.md)）。
  - 理由: 初回準備の手順 1 が `allcrates/` を作り直すので、やり直せば直る。

## 検討した他の案

- 案: 実行の前に `allcrates/` の有無を確かめ、無ければ初回準備を始める
  - 採用しなかった理由: ビルドの失敗から判定する既存の方針（ADR 0105）と別の仕組みが増える。失敗の判定に足せば、ほかの状態と同じ処理で済む。

## 影響

- 文が出る場所と文言は cargo の版で変わりうる。確かめたのは Issue に書かれた出力（cargo 1.100.0 系）の形である。
