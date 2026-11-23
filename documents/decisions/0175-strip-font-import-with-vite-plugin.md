---
type: Decision
title: "ADR 0175"
description: "デザインシステムの `bundle.css` の、書体を外部から読み込む `@import` の行を、Vite のプラグインで取り除いて読み込み、教材チェックツールの描画でも同じように取り除く"
status: deprecated
superseded_by: /decisions/0197-remove-font-import-from-design-system-css.md
---

- 日付: 2026-11-23
- 関連仕様: [画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステムの CSS とアイコン」
- 置き換え: なし

## 背景

デザインシステムの `components/bundle.css` の先頭には、書体を外部のサイトから読み込む `@import` の行がある。アプリは、この行をデザインシステムのファイルから消さずに、ビルドのときに取り除いて読み込む（[ADR 0153](0153-load-design-system-css-directly-from-repository.md)）。取り除く方法（ビルドの仕組みのどこで行うか）は、実装のときに決めるとして、未決事項に残っていた。実装計画の M1 で、デザインシステムの CSS を読み込むために決める必要があった。

アプリのフロントエンドは Vite でビルドする（[アプリの技術構成](../spec/architecture/tech-stack.md)）。教材チェックツールの描画のチェックは、Remotion のバンドルで動画を描く（[教材チェックツール](../spec/content-guidelines/check-tool.md)）。

## 決定

- アプリは、Vite のプラグインで、デザインシステムの `components/bundle.css` から書体の `@import` の行を取り除く。開発時の起動と本番ビルドの両方で取り除く（[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステムの CSS とアイコン」）。Claude が決めた。
  - 理由: CSS は Vite が読み込むので、その途中で取り除けば、デザインシステムのファイルを変えずに、写しも作らずに済む。開発時の起動でも取り除けば、開発時と本番ビルドで読み込む書体が変わらない。
- 教材チェックツールの描画のチェックでも、同じように取り除く。Claude が決めた。
  - 理由: 描画の結果が、外部のサイトに届くかどうかで変わらないようにする。

## 検討した他の案

- 案: `bundle.css` から `@import` の行を取り除いた写しを、ビルドの前に作る
  - 採用しなかった理由: ADR 0153 が、写しを作らないと決めている
- 案: CSP で外部の読み込みを拒否するだけにし、行は残したまま読み込む
  - 採用しなかった理由: 開発時の起動では CSP が掛からず（[音声をアプリ同梱データから asset:// で読む再生の技術検証](../verification/issue-46-serve-narration-via-asset-protocol.md)「開発時の起動（`tauri dev`）でも読めるか」）、開発時だけ外部の書体を読み込んでしまう。本番ビルドでは、拒否のたびに CSP の違反が出る

## 影響

- デザインシステムの `bundle.css` の先頭の行の書き方が変わったら、プラグインを直す必要がある。
