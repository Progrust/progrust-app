---
type: Convention
title: "index.md の書き方と更新"
description: "各ディレクトリの `index.md` に書ける行（見出し行と `* [title](パス) - description` の行）、下位ディレクトリの載せ方、concept を追加・削除・改名したときの `index.md` の直し方、バンドルのルートの `index.md` の frontmatter を定める。"
---

- concept を置くディレクトリには、必ず `index.md` を置く。`index.md` は、そのディレクトリの目次になる。
- `index.md` には、そのディレクトリの concept と、1段下のディレクトリをすべて1行ずつ載せる。同じものを2回載せない。asset は載せない。

## 書ける行

`index.md` の本文に書けるのは、次の3種類の行だけ。最初の行は見出し行にする。

- 見出し行: `# ` で始まる行。`##` 以下は使わない。
- 空行
- 項目の行: `* [名前](パス) - 説明`

項目の行は、載せるものによって次のように書く。

- 同じディレクトリの concept: `* [title](ファイル名) - description`。名前と説明は、その concept の frontmatter の `title` と `description` の値をそのまま書く。
- 1段下のディレクトリ: `* [名前](ディレクトリ名/index.md) - 説明`。パスはディレクトリ名に `/index.md` を付けて書き、`./` を付けない。説明には、そのディレクトリに何があるかを1文で書く。

```markdown
# 質疑応答

* [質疑応答の目的とスコープ](scope.md) - 質疑応答の仕様が扱う範囲と、`claude -p` の呼び出し方の共通ルールの参照先を定める。
* [質疑応答の会話履歴](history.md) - 質疑応答の会話履歴を保持する範囲と、AI への入力に含める往復数を定める。
```

```markdown
# 仕様

* [質疑応答](qa-chat/index.md) - 動画や解説について、チャット形式で AI に質問する機能を定める。
```

- `decisions/index.md` の見出しの分け方は [ADR の運用ルール](decision-records.md) に従う。

## 更新

- concept を追加・削除・移動・改名したときや、concept の `title`・`description` を変えたときは、同じ変更の中で、そのディレクトリの `index.md` を直す。
- ディレクトリを追加・削除・改名したときは、1段上の `index.md` も直す。

## バンドルのルートの index.md

- frontmatter を持てる `index.md` は、バンドルのルートの `documents/index.md` だけ。ルート以外の `index.md` には frontmatter を書かない。
- ルートの `index.md` の frontmatter には、次の3つのキーをこの順に書く。値はすべて二重引用符で囲む。

| キー | 値 |
| --- | --- |
| `okf_version` | `"0.3"` |
| `title` | バンドルの名前 |
| `description` | このバンドルが何を扱い、いつ使うかの1文 |
