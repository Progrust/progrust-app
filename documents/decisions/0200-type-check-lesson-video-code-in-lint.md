---
type: Decision
title: "ADR 0200"
description: "TypeScript の型検査（`npm run lint` の `tsc --noEmit -p tsconfig.json`）の対象に、教材の動画のコード（`content/lessons/*/video/` の `.ts`・`.tsx`）を加える"
---

- 日付: 2026-11-23
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)「lint と整形」、[教材チェックツール](../spec/content-guidelines/check-tool.md)
- 置き換え: なし

## 背景

教材の動画のコード（`content/lessons/{lesson_id}/video/` の `.ts`・`.tsx`）は、どの機械のチェックでも型検査されていなかった（Issue #150）。

- `npm run lint` の型検査（`tsc --noEmit -p tsconfig.json`）は、`tsconfig.json` の `include` が `src` と教材チェックツールの描画用のファイルだけで、`content/` を含まなかった。
- 教材チェックツールの「lint」は禁止事項（[動画のコードの禁止事項](../spec/content-format/video/prohibited-code.md)）だけを検査し、「描画」はバンドルで型を取り除くので、型の誤りは描画したフレームで例外になったときにしか見つからない。
- アプリのビルド（Vite）も型を検査しない。

そのため、共通部品（`src/video/`）の props の名前や型を誤って書いた動画のコードや、共通部品の型を変えたときに合わなくなった既存のレッスンの動画のコードに、気づけなかった。

## 決定

`tsconfig.json` の `include` に、`content/lessons/*/video/**/*.ts` と `content/lessons/*/video/**/*.tsx` を加える。教材の動画のコードは、アプリのコードと同じ設定で、`npm run lint` の型検査を受ける。Claude が技術的な判断で決めた。

- 理由: 動画のコードはアプリのコードと同じく TSX で、共通部品をアプリのコードから import し、アプリのビルドに含まれる（[ADR 0062](0062-write-video-scenes-in-unrestricted-tsx.md)「影響」）。同じ設定で型検査すれば、共通部品の型と合わないことが分かる。
- 理由: `npm run lint` は `npm run check` に含まれ、CI と実装のループの機械のチェックの両方で実行される。新しい検査の仕組みを作らずに、どちらでも検査される。
- 見本の教材（ch04-01・ch04-04）は、加えた後も型検査を通った。共通部品に無い props を渡した動画のコードは、型検査でエラーになった。

## 検討した他の案

- 案: 教材チェックツール（`npm run content:check`）に、型検査の項目を加える
  - 採用しなかった理由: 型検査は、TypeScript・TSX の lint として `npm run lint` で行うと決めている（[開発環境と依存の版](../spec/architecture/development-environment.md)「lint と整形」）。教材チェックツールで `tsc` を別に呼ぶと、型検査の設定と実行の場所が 2 つになる
- 案: 教材の動画のコード用に、別の tsconfig を作る
  - 採用しなかった理由: 動画のコードはアプリのビルドに含まれるので、アプリのコードと違う設定で型検査する理由がない

## 影響

- 教材の動画のコードに型の誤りがあると、`npm run lint`（`npm run check`）が失敗する。教材を書いたら、`npm run content:check` に加えて `npm run lint` も通す必要がある。
- 型検査の設定（`strict`、`noUnusedLocals` など）は、動画のコードにもそのままかかる。使っていない変数や引数も、エラーになる。
- 禁止事項の検査（ESLint の独自の規則など）は、これまでどおり教材チェックツールの「lint」で行う。アプリの ESLint（`eslint src tools`）は、動画のコードを検査しない。
