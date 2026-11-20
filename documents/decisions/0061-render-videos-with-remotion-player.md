---
type: Decision
title: "ADR 0061"
description: "動画を、書き出したファイルにせず、アプリの中で Remotion の Player で描画する"
---

- 日付: 2026-11-19
- 関連仕様: [動画を構成するファイル](../spec/content-format/video/structure.md)、[アプリの技術構成](../spec/architecture/tech-stack.md)、[フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)、[画面の見た目とテーマ](../spec/screens/appearance.md)、[動画の色の指定](../spec/content-format/video/colors.md)、[動画のシーンの書き方](../spec/content-format/video/scenes.md)
- 置き換え: なし

## 背景

動画を、書き出した動画ファイルとして再生するか、アプリの中で Remotion の部品として描画するかを決める必要があった。テーマ（ライト／ダーク）や章カラーへの追従、配布サイズ、オフライン動作に関わる。
試作は確認用に動画ファイル（mp4）に書き出した。1 分 44 秒で 6.9MB だった。動画ファイルにすると、テーマと章カラーが焼き込みになり、ライトとダークを両方持つと 2 倍になる。アプリの中で描画すれば、同梱するのは音声だけになる。

## 決定

- 動画はファイルに書き出さず、アプリの中で Remotion の Player（`@remotion/player`）で描画する（`content-format.md`「構成」、`architecture.md`「技術構成」）。
- テーマと章カラーは、アプリが再生時に指定する（`content-format.md`「構成」）。

## 検討した他の案

- 案: 書き出した動画ファイルを再生する
  - 採用しなかった理由: 動画は容量が重い。書き出しに時間がかかる。Remotion には Player があるので、Player を別途用意する必要がない

## 影響

- 動画のコード（TSX）はフロントエンドのビルドに含まれる（`architecture.md`「役割分担」）。
- テーマを切り替えると、再生中の動画の色も、再生を止めずに切り替わる（`screens.md`「テーマ」）。同じ動画は、再生するときに指定した章の章カラーで表示される（`content-format.md`「色の指定」）。
- 再生位置を移せるように、シーンは現在のフレームだけから絵が決まるように書く必要がある。CSS のアニメーションとトランジション、タイマー、乱数は使えない（`content-format.md`「シーン」）。
- アプリが Remotion に依存する。Remotion のライセンスは未確認である（`non-functional.md`「未決事項」）。
