---
type: Specification
title: "教材のディレクトリ構成"
description: "`content/` の下の `chapters.json`・`elements.json`・`voice.json`・`readings.json`・`terms.json`・`assets/`・`lessons/`（レッスンごとの `lesson.json`・`video/`・`exercise/`）の配置と、動画の共通部品と音声を作るツールを `content/` の外に置くことを定める。"
---

リポジトリの `content/` に置き、アプリに同梱する。同梱のしかたは [フロントエンドとバックエンドの役割分担](../architecture/responsibilities.md)・[データの保存場所](../architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」 に定める。

```
content/
├── chapters.json                   # 章とレッスンの並び（spec/content-format/chapters-json.md）
├── elements.json                   # 要素マスタ（spec/content-format/elements-json.md）
├── voice.json                      # 声の設定（spec/content-format/video/voice.md）。全レッスン共通
├── readings.json                   # 読みの対応表（spec/content-format/video/voice.md）。全レッスン共通
├── terms.json                      # 訳語の表（spec/content-format/terms-json.md）。全レッスン共通
├── assets/                         # 全レッスン共通の画像
└── lessons/
    └── {lesson_id}/
        ├── lesson.json             # レッスン定義（spec/content-format/lesson-json.md）
        ├── video/                  # 動画（spec/content-format/video/index.md）。章末復習レッスンにはない
        │   ├── script.json         # 台本（spec/content-format/video/script-json.md）
        │   ├── index.tsx           # 動画の入口（spec/content-format/video/scenes.md）
        │   ├── …                   # シーンとレッスン専用の部品（.tsx / .ts / .css）
        │   ├── voice/              # 音声。1 文 1 ファイル（spec/content-format/video/voice.md）
        │   ├── durations.json      # 音声の長さ（spec/content-format/video/voice.md）
        │   └── cue-reference.json  # 合図を合わせたときの音声の長さ（spec/content-format/video/timeline-and-cues.md）
        ├── assets/                 # このレッスン専用の画像
        └── exercise/               # 演習（spec/content-format/exercise.md）。動画のみのレッスンにはない
            ├── problem.md
            ├── explanation.md
            ├── starter/            # 初期コード（C のレッスンは starter/main.c だけ）
            ├── solution/           # 模範解答（C のレッスンは solution/main.c だけ）
            ├── cases.json          # judge.mode = stdout の場合（C のレッスンは常にこれ）
            └── judge_tests.rs      # judge.mode = test の場合（C のレッスンにはない）
```

- C のレッスンの演習のファイルは [演習のファイル形式](exercise.md)「C の演習」に定める。

- 動画の共通部品（[動画のシーンの書き方](video/scenes.md)）と音声を作るツール（[動画の音声](video/voice.md)）は `content/` の外に置く（[リポジトリ構成](../architecture/repository.md)）。
