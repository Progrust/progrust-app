---
type: Convention
title: "frontmatter の書き方"
description: "concept の frontmatter で使うキー（`type`・`title`・`description`・`status`・`superseded_by`）とその順序、値の引用符、6つの `type` の語彙と使い分け、使わないキーを定める。"
---

- concept は、1行目から frontmatter を書く。`---` の行で始め、`---` の行で閉じる。
- 使うキーは次の5つだけ。この順に書く。

| キー | 必須か | 書き方 |
| --- | --- | --- |
| `type` | 必須 | 下の6つのどれか。引用符で囲まない |
| `title` | 必須 | 二重引用符で囲む |
| `description` | 必須 | 二重引用符で囲む |
| `status` | ADR だけ | 書き方は [ADR の運用ルール](decision-records.md) |
| `superseded_by` | ADR だけ | 書き方は [ADR の運用ルール](decision-records.md) |

- `title` と `description` の値は、必ず二重引用符で囲む（JSON の文字列と同じ書き方）。値の中の `"` は `\"` と書く。
- `title` と `description` の中身の決め方は [文書の分割単位と名前の付け方](splitting.md) に従う。
- バンドルのルートの `index.md` の frontmatter は [index.md の書き方と更新](index-files.md) に従う。ルート以外の `index.md` は frontmatter を持たない。

```markdown
---
type: Specification
title: "質疑応答の会話履歴"
description: "質疑応答の会話履歴を保持する範囲と、AI への入力に含める往復数を定める。"
---
```

## type の語彙

`type` は次の6つに限る。新しい type を作らない。

| type | 入る文書 | エージェントが引くとき |
| --- | --- | --- |
| `Specification` | 現時点の仕様そのもの。機能の目的とスコープ（`scope.md`）もこれ | 実装するとき |
| `Acceptance Criteria` | 機能の受け入れ条件（`acceptance.md`） | 実装を確かめるとき |
| `Open Questions` | 機能の仕様でまだ決まっていない事項（`open-questions.md`） | 仕様を変える前 |
| `Glossary` | 用語集 | 言葉の意味を確かめるとき |
| `Decision` | ADR | 仕様を変える前 |
| `Convention` | `documents/` の中の文書の書き方と運用ルール、AI エージェントの Issue の起票の決まり | 文書を書くとき、Issue を起票するとき |

- 教材の作成ルール（`spec/content-guidelines/`）は、プロダクト側の決まりなので `Specification` にする。`Convention` は、`documents/` の中の文書の書き方と、AI エージェントの Issue の起票の決まり（[Issue の起票](issues.md)）だけに使う。
- 領域（画面、教材、デザインなど）は type で分けない。置き場所（ディレクトリ）で分ける。

## 使わないキー

OKF が定める次のキーは使わない。

- `verified`（`human:` の actor も使わない）
- `stale_after`
- `generated`
- `sources`
- `tags`
- Attested Computation の各キー
