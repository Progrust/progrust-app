---
type: Specification
title: "データの互換性"
description: "`schema_version` による形式の変換、変換前ファイルの退避、教材の更新時に過去の記録を残す扱いを定める。"
---

- アプリが管理するファイルはすべて `schema_version` を持つ。
- `settings.json`・`progress.json`・`jobs.json` は、アプリ起動時に古いバージョンを検出したら自動で新しい形式に変換する。変換前のファイルは `.v{旧バージョン}.bak` として残す。
- 学習記録・分析ファイルの扱いは [学習記録・分析ファイルのスキーマのバージョン管理](../learning-record-and-analysis/schema-version.md) に従う。
- 教材の更新でレッスンが削除・変更されても、過去の学習記録・試行ログは削除しない。存在しないレッスンの進捗は表示しない。
