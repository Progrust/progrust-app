---
type: Decision
title: "ADR 0207"
description: "下書き（`drafts/*.json`）と試行ログ（`attempts/*.json`）に `schema_version`（いまは 1）を持たせ、読み込めないときは `.broken-{日時}` を付けて退避し、作り直さずに無いものとして扱う"
---

- 日付: 2026-11-23
- 関連仕様: [演習コードの下書き](../spec/lesson-flow/drafts.md)、[試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)「読み込めない試行ログ」、[データの保存場所](../spec/architecture/data-locations.md)「書き込みの規則」、[データの互換性](../spec/non-functional/data-compatibility.md)
- 置き換え: なし

## 背景

[データの互換性](../spec/non-functional/data-compatibility.md) は「アプリが管理するファイルはすべて `schema_version` を持つ」とするが、下書きと試行ログの形式には `schema_version` が無く、非機能要件の未決事項（実装のときに決めること）になっていた。[ADR 0074](0074-do-not-log-video-viewing.md) は、試行ログの `schema_version` を 1 のままにするとしていたが、試行ログのスキーマには `schema_version` の項目が無かった。読み込めないファイルの退避の決まり（[ADR 0185](0185-name-temp-and-broken-files-and-define-unreadable.md)）も、`settings.json`・`progress.json`・`jobs.json` だけを対象にしていた。実装計画の M2 で、下書きと試行ログを初めて書くので、決める必要があった。

## 決定

Claude が決めた。

- 下書きと試行ログに `schema_version` を持たせる。いまの版は `1` とする。
  - 理由: 試行ログは消さずに残し（[ADR 0029](0029-keep-full-attempt-logs.md)）、後の版のアプリと学習記録のジョブが読む。形式を変えたときに、どの版で書いたかが分からないと変換できない。下書きも、アプリの版を上げた後に読む。データの互換性の決まりとそろう。
- 読み込めない下書き・試行ログ（[データの保存場所](../spec/architecture/data-locations.md)「書き込みの規則」と同じ条件）は、`.broken-{日時}` を付けて退避し、作り直さずに、そのファイルが無いものとして扱う。下書きなら初期コードを表示し、試行ログなら、未完了のセッションが無いものとして新しいセッションを始める。
  - 理由: `settings.json` などと違い、下書きと試行ログはレッスンやセッションごとのファイルで、無い状態が正しい初期状態である。退避すれば、壊れたファイルの中身は残り、学習者が手で直せる。試行ログは消さない決まりなので、上書きせず、退避して残す。
- 退避したときのユーザーへの通知は、`settings.json` などと同じく決めずに、アーキテクチャの未決事項に加えた。
  - 理由: 通知するかどうかは、学習者が見るものなので、Claude だけでは決めない。

## 検討した他の案

- 案: 下書きには `schema_version` を持たせない（正解すると消える、短い間だけのファイルなので）
  - 採用しなかった理由: 正解するまでは、アプリの版を上げた後も残る。ファイルごとに決まりを変えると、読み書きの部品を共通にしにくい
- 案: 読み込めない下書きを、退避せずに上書きする
  - 採用しなかった理由: 手で直せたかもしれないコードが消える

## 影響

- 生成問題の `meta.json` と `prepared.json` の `schema_version` は、まだ決めていない（非機能要件の未決事項）。
