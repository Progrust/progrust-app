---
type: Convention
title: "ドキュメントの検査"
description: "`documents/` を変更したときに `python3 tools/check-documents.py` を実行すること、仕様の文書を移したり名前や小見出しを変えたりしたときに `python3 tools/design-system.py check` も実行すること、出力と終了コードの読み方、検査する項目と規則名、指摘の直し方、公式の validator（`tools/okf/validate.py`）の置き場所を定める。"
---

## 実行

- `documents/` を変更したら、リポジトリのルートで次のコマンドを実行し、指摘をすべて直す。

```bash
python3 tools/check-documents.py
```

- `documents/spec/` の文書を移したり、名前や小見出しを変えたりしたときは、`python3 tools/design-system.py check` も実行し、指摘をすべて直す。デザインシステムの README などからの、仕様への参照を確かめるためである（[デザインシステムのファイルと変更の手順](../spec/screens/design-system-files.md)「検査」）。この検査は `documents/` の外のファイルを見るので、`check-documents.py` では見つからない。
- 引数を省略すると、リポジトリの `documents/` を検査する。引数にバンドルのルートを渡すと、そのディレクトリを検査する。
- 標準ライブラリだけで動く（Python 3.9 以降）。
- Claude Code では、応答を終えるときに、リポジトリの `.claude/settings.json` の hook が `--hook` を付けてこの検査を実行する。指摘があると、応答を終える前に指摘が示されるので、すべて直す。hook は、同じ応答の中で2回目以降は検査しない。指摘が残っていないことは、上のコマンドを自分で実行して確かめる。

## 出力

- 指摘は `パス:行: 規則名: 説明` の形で1行ずつ出る。最後に、検査した `.md` の数と指摘の件数が出る。
- 終了コードは、指摘なしが 0、指摘ありが 1、バンドルのルートが無いときと引数が誤っているときが 2。

## 指摘の直し方

- 指摘の説明には、直し方が書いてある。それに従って、指摘された文書を直す。
- `index-entry-text` の指摘は、`index.md` の行を concept の `title`・`description` に合わせて直す。
- `link-target` と `bundle-path` の指摘は、リンク先とパスを実在するファイルに直す。ファイルを移動・改名したときの直し方は [参照とリンクの書き方](links.md) に従う。
- 指摘が無くても、運用ルールをすべて満たしたことにはならない。検査しない決まり（本文のリンクを相対パスで書くこと、ADR から仕様への片方向のリンク、ADR の `title` の形、見出しの番号、`description` の文の形など）は、各運用ルールを読んで守る。

## 検査する項目

| 規則名 | 検査する内容 |
| --- | --- |
| `encoding` | UTF-8 で読めること。先頭に BOM が無いこと |
| `file-name` | `.md` のファイル名とディレクトリ名が英小文字・数字・ハイフンだけであること。拡張子が小文字の `.md` であること。大文字小文字だけが違う名前が無いこと |
| `file-kind` | `README.md`・`log.md` が無いこと。frontmatter の無い `.md`（`index.md` 以外）が無いこと |
| `index-missing` | 配下に `.md` を持つディレクトリ（ルートを含む）に `index.md` があること |
| `frontmatter-syntax` | frontmatter が `---` で閉じていること。各行が `キー: 値` の1行で、コロンの後に空白がちょうど1つあること。空行・コメント・字下げ・行末の空白・リスト・入れ子・複数行の値・重複したキーが無いこと |
| `frontmatter-keys` | concept のキーが `type`・`title`・`description`・`status`・`superseded_by` だけで、この順に並ぶこと。`type`・`title`・`description` があること |
| `type-value` | `type` が7つの語彙のどれかで、引用符で囲んでいないこと |
| `string-value` | `title`・`description` が二重引用符で囲んだ JSON の文字列で、空でなく、改行やタブなどの制御文字を含まないこと |
| `status-value` | `status` が `type: Decision` の concept にだけあり、値が `draft`・`stable`・`deprecated` のどれかで、引用符で囲んでいないこと |
| `superseded-by` | `superseded_by` が `type: Decision` で `status: deprecated` の concept にだけあり、引用符なしの `/` 始まりのパスで、自分以外の実在する concept を指すこと |
| `root-index-frontmatter` | ルートの `index.md` に frontmatter があり、キーが `okf_version`・`title`・`description` の3つだけでこの順に並び、`okf_version` が `"0.3"` であること |
| `index-frontmatter` | ルート以外の `index.md` に frontmatter が無いこと |
| `index-line` | `index.md` の本文が `# ` の見出し行で始まり、見出し行・空行・`* [名前](パス) - 説明` の行だけでできていること |
| `index-target` | `index.md` の項目のパスが、同じディレクトリの concept か、1段下のディレクトリの `index.md` であること |
| `index-coverage` | `index.md` に、同じディレクトリの concept と、配下に `.md` を持つ1段下のディレクトリがすべて1回ずつ載り、実在しないものが載っていないこと |
| `index-entry-text` | concept の項目の名前と説明が、その concept の `title`・`description` と一致すること |
| `link-target` | 本文のリンク先がバンドルの中に実在すること。外部へのリンクは `http:`・`https:`・`mailto:` だけであること |
| `bundle-path` | コードブロックの中とインラインコードの中（`title`・`description` のインラインコードを含む）に書いた `spec/`・`decisions/`・`conventions/`・`verification/` で始まる `.md` のパスが、バンドルのルートから見て実在すること。`*`・`{`・`<`・`…` を含むパスは調べない |
| `section-sign` | 節番号の記号（`§`）が、本文・`title`・`description`・コードブロックの中に無いこと。インラインコードの中は調べない |
| `footnote` | 脚注の記法（`[^1]` など）が無いこと |
| `work-reference` | 一時作業用ディレクトリを `./` から始まる相対パスで書いた参照が無いこと（それ以外の書き方は検出しない） |
| `heading-level` | concept の本文に深さ1の見出し（`# ` や `===` の下線）が無いこと |
| `fence-unclosed` | コードフェンスが閉じていること |

- 名前が `.` で始まるファイルとディレクトリは検査しない。

## 公式の validator

- OKF の公式の validator は `tools/okf/validate.py` にある。実行方法は `tools/okf/README.md` にある。
- 公式の validator は hook では実行せず、OKF の版を上げるときなど必要なときに手動で実行する。
