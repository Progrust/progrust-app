---
type: Specification
title: "動画の音声"
description: "VOICEPEAK で作る音声ファイルの形式と置き場所、声の設定（`voice.json`）、読みの対応表（`readings.json`）、音声の長さ（`durations.json`）、音声を作るツールの動作を定める。"
---

- ナレーションの音声は、教材の作者の環境で、VOICEPEAK の CLI から作る。作った音声はリポジトリに入れ、アプリに同梱する。学習者の環境に VOICEPEAK は要らない。CI では音声を作らない。
- 音声は 1 文 1 ファイルとし、`voice/{文の ID}.m4a` に置く。形式は AAC、モノラルとし、ビットレートは 64kbps を目安とする。

**声の設定（`content/voice.json`）**

```json
{
  "narrator": "Natsuki Karin",
  "speed": 120,
  "pause": 0.2,
  "emotion": { "hightension": 20, "buchigire": 5, "nageki": 0, "sagesumi": 20, "sasayaki": 80 }
}
```

| フィールド | 説明 |
|---|---|
| `narrator` | VOICEPEAK のナレーター名。`Natsuki Karin` は夏色花梨である |
| `speed` | 読む速さ（VOICEPEAK の速さの値） |
| `pause` | 「、」の間の長さ（秒。0 以上） |
| `emotion` | 感情の項目名と値。項目はナレーターごとに決まる |

- 声の設定は全レッスンで共通とする。
- 声の設定を変えたら、全レッスンの音声を作り直す。

**読みの対応表（`content/readings.json`）**

```json
{
  "Rust": "ラスト",
  "String": "ストリング",
  "値": "あたい"
}
```

- 読み間違えやすい語と、`say` に書く読みの対応を、全レッスン共通で持つ。キーが語、値が読みである。
- 作者は `say` を書くとき、この表に従う（[動画の作成ルール](../../content-guidelines/video.md)「台本」）。音声を作るツールは `say` を書き換えない。

**音声の長さ（`durations.json`）**

```json
{
  "m1": {
    "seconds": 3.757,
    "say": "二行目で、変数エスワンにストリングのあたいを入れました。",
    "voice": {
      "narrator": "Natsuki Karin",
      "speed": 120,
      "pause": 0.2,
      "emotion": { "hightension": 20, "buchigire": 5, "nageki": 0, "sagesumi": 20, "sasayaki": 80 }
    }
  }
}
```

| フィールド | 説明 |
|---|---|
| `seconds` | その文の音声ファイル（`voice/{文の ID}.m4a`）の長さ（秒） |
| `say` | その音声を作ったときの `say` |
| `voice` | その音声を作ったときの声の設定（`voice.json` と同じ形） |

- キーは文の ID とする。台本のすべての文について 1 件ずつ持つ。
- 音声を作るツールが書く。作者は手で書き換えない。

**音声を作るツール**

`tools/` に置く。作者の環境の VOICEPEAK と ffmpeg を使う。

- 台本の文を 1 文ずつ順に VOICEPEAK に渡して、音声を作る。VOICEPEAK を同時に 2 つ以上動かさない。
- `durations.json` の `say`・`voice` が台本の `say`・`voice.json` と同じで、音声ファイルがある文は、作り直さない。すべての文を作り直す指定もできる。
- `say` が 140 字を超える文があるときは、エラーにして音声を作らない。
- 「、」の間は次のように作る。`say` をそのまま VOICEPEAK に渡して音声を作り、できた音声の中の「、」にあたる無音を `pause` 秒まで切り詰める。`pause` 秒より短い無音と、文の途中の「。」にあたる無音は、そのままにする。
- 切り詰めたあとの音声を `.m4a` に変換し、変換後のファイルの長さを `durations.json` の `seconds` に書く。
- 台本にない文の音声ファイルと、`durations.json`・`cue-reference.json` の記録は消す。
- `cue-reference.json` の書き方は [動画のタイムラインと合図](timeline-and-cues.md) に定める。
