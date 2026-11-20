# OKF の公式 validator

`validate.py` は、Open Knowledge Format（OKF）の公式リポジトリにある適合チェック用スクリプトを、変更せずにコピーしたもの。

- 取得元: https://github.com/open-knowledge-format/open-knowledge-format/blob/main/skills/okf-generate/scripts/validate.py
- 取得したコミット: `c4df7bab129884a19673c7098dd8a5c55bf06c86`（2026-10-27）
- ライセンス: Apache License 2.0（`LICENSE`）
- 対象の仕様: OKF v0.3

## 実行方法

Python 3.9 以降と PyYAML が必要。uv があれば、次のコマンドで PyYAML を一時的に用意して実行できる。

```bash
uv run --no-project --with pyyaml python tools/okf/validate.py documents --strict
```

`--strict` を付けると、警告があるときも終了コード 1 になる。

## 更新するとき

取得元の新しい版で `validate.py` を置き換え、このファイルのコミットと日付を書き換える。中身は編集しない。
