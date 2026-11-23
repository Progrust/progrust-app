---
type: Decision
title: "ADR 0179"
description: "`settings.json` が無いときの初期値を定め、テーマは「システムに合わせる」、エディタの文字サイズは 14px、動画は音量 1.0・ミュートなし・1 倍・字幕なしにする"
---

- 日付: 2026-11-23
- 関連仕様: [設定画面と settings.json](../spec/screens/settings-screen.md)「settings.json」
- 置き換え: なし

## 背景

`settings.json` の形は例で示してあるが、ファイルが無いとき（初めて起動したとき、退避して作り直したとき。[データの保存場所](../spec/architecture/data-locations.md)「書き込みの規則」）にどの値を使うかは、仕様に無かった。モデルの既定値（[AI モデルの選択](../spec/claude-integration/model-selection.md)）、字幕を既定では表示しないこと（[動画画面](../spec/screens/video-screen.md)）、初回起動画面を出す条件（[初回起動画面](../spec/screens/first-launch.md)）、エディタの文字サイズの既定が 14px であること（デザインシステム「Progrust」の README「書体」）は、それぞれ別の場所に書かれていた。実装計画の M1 で、設定を読み書きするために決める必要があった。

## 決定

- `settings.json` が無いときの初期値を定めた（[設定画面と settings.json](../spec/screens/settings-screen.md)「settings.json」）。Claude が決めた。
  - モデル、字幕、`first_launch_completed`、エディタの文字サイズは、既に別の場所に書かれた既定にそろえた。
  - テーマは「システムに合わせる」（`system`）にした。理由: 学習者が OS で選んだ見た目に、何も設定せずに合う。
  - 音量は 1.0、ミュートなし、再生速度は 1 倍にした。理由: 動画を作ったときのそのままの速さと音量で始まる。
  - `tool_paths` はどちらも `null`（自動検出）にした。理由: `null` が自動検出を表す（[設定画面と settings.json](../spec/screens/settings-screen.md)「settings.json」）。

## 検討した他の案

- 案: テーマの初期値をライトにする
  - 採用しなかった理由: OS をダークにしている学習者が、最初にまぶしい画面を見ることになる。「システムに合わせる」なら、どちらの学習者にも合う

## 影響

- 設定を読み込めずに作り直したときも、この初期値になる。
