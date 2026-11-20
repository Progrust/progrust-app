---
type: Verification Report
title: "claude -p の隔離起動の技術検証"
description: "claude -p を、ユーザー設定を読み込まず、ツールを使わせず、学習者のログイン状態のまま起動できるかを Issue #8 で検証した結果（フラグの組み合わせごとの効き方、--bare が OAuth を読まないこと、ツール無効の確認、設定ディレクトリに残るもの、推奨のフラグと再実行の 1 行のコマンド）を記す。"
---

- 日付: 2026-11-20
- Issue: [#8](https://github.com/Progrust/progrust-app/issues/8)
- 確かめた OS: macOS 27.2.1（Apple Silicon、arm64）
- 関連仕様: [claude -p の呼び出し](../spec/claude-integration/invocation.md)、[Claude Code 連携の設計方針](../spec/claude-integration/design-policy.md)、[ADR 0020](../decisions/0020-ai-returns-output-app-writes.md)、[ADR 0021](../decisions/0021-isolated-claude-invocation.md)

## 環境

| 項目 | 値 |
|---|---|
| OS | macOS 27.2.1、Apple Silicon（arm64） |
| Claude Code | 2.1.329（ネイティブインストーラーで導入） |
| ログイン | claude.ai のサブスクリプション（OAuth。`claude auth status` で `authMethod: "claude.ai"`）。API キーは使っていない（`apiKeySource: "none"`） |
| rustc / cargo | 1.99.1 / 1.99.1 |
| node | v25.9.0 |
| 測定時の負荷 | load average 18〜19（他のエージェントが同時に動いていた。時間の厳密な比較はしていない） |

入れ子で `claude` を実行するため、検証を動かしている環境から引き継いだ `CLAUDE*`・`AI_AGENT`・`MCP_CONNECTION_NONBLOCKING` などの環境変数を取り除いて実行した（`env -u ...`）。`claude -p` の呼び出しは合計 33 回程度で、API まで届いたのは約 18 回。モデルはすべて `haiku`。

## 方法

- 作業ディレクトリは、OS の一時ディレクトリの下に `mktemp -d` で作った。リポジトリの中を cwd にすると、Claude Code は git のルートをプロジェクトとみなす（自動メモリーのパスがリポジトリ名になった）。アプリが OS の一時ディレクトリを使う仕様は、この点でも正しい。
- 読み込まれた設定の判定は、モデルの答えだけに頼らず、機械的に見られるものを使った。
  - `--output-format stream-json --verbose` の最初の行の `system/init` イベント（`tools`、`mcp_servers`、`skills`、`slash_commands`、`agents`、`plugins`、`output_style`、`memory_paths`、`apiKeySource` を持つ）
  - フック・MCP サーバーが起動したときに書くファイル（副作用）
  - `--debug-file` のログ
  - CLAUDE.md・メモリーは、仕込んだ合言葉をモデルに答えさせて確かめた
- 「読み込まれない」の判定は、同じ目印が隔離のフラグなしでは効くこと（対照の実行）を確かめたうえで行った。

### 確かめたフラグ（`claude --help`、2.1.329）

| フラグ | ヘルプの説明（要約） |
|---|---|
| `--tools ""` | 使えるようにする組み込みツール。`""` で無し |
| `--safe-mode` | CLAUDE.md・スキル・インストールしたプラグイン・フック・MCP サーバー・カスタムコマンドとエージェント・出力スタイルなどをすべて無効にする。管理者の設定（managed settings）と管理プラグインのフックは適用される。認証・モデル選択・組み込みツールは通常どおり |
| `--setting-sources <sources>` | 読み込む設定の出どころ（user, project, local）。`""` を渡せた |
| `--strict-mcp-config` | `--mcp-config` 以外の MCP 設定を無視する |
| `--disable-slash-commands` | スキルとコマンドをすべて無効にする |
| `--bare` | 最小モード。フック・CLAUDE.md の自動探索などを飛ばすが、認証は `ANTHROPIC_API_KEY` か `--settings` の apiKeyHelper だけ（OAuth とキーチェーンは読まない） |
| `--no-session-persistence` | セッションをディスクに保存しない（`-p` のときだけ） |
| `--restricted` | コマンド実行系のツールを外し、user/project/local の設定ファイルを無視する（試していない。下の組み合わせで足りたため） |

### 仕込んだ目印

(a) プロジェクトの設定として、空の一時ディレクトリに置いた。

| 種類 | 置いたもの | 目印 |
|---|---|---|
| CLAUDE.md | `CLAUDE.md`、`CLAUDE.local.md` | 合言葉 `PROJ-CLAUDEMD-7731`、`LOCAL-MD-4410` |
| フック | `.claude/settings.json` の `SessionStart`・`UserPromptSubmit`（ファイルを書き、`UserPromptSubmit` は追加コンテキスト `HOOKCTX-5520` を返す） | 副作用ファイル、合言葉 |
| MCP サーバー | `.mcp.json` の stdio サーバー（Python。起動時にファイルを書き、孫プロセス `sleep` を起動する）と、`.claude/settings.local.json` の `enableAllProjectMcpServers: true` | init の `mcp_servers`、副作用ファイル |
| スキル・エージェント・コマンド・出力スタイル | `.claude/skills/`、`.claude/agents/`、`.claude/commands/`、`.claude/output-styles/`（設定の `outputStyle` に指定） | init の各一覧、合言葉 `SKILL-8802`・`STYLE-6604` |
| メモリー | `.claude/settings.local.json` の `autoMemoryDirectory` で、作業用のディレクトリの `MEMORY.md` を指す | 合言葉 `MEMORY-1301` |

(b) ユーザーの設定として、`CLAUDE_CONFIG_DIR` に別の設定ディレクトリを用意した。`settings.json` のフック（SessionStart・UserPromptSubmit）と `outputStyle`、`CLAUDE.md`、`skills/`、`agents/`、`commands/`、`output-styles/`、`.claude.json` の `mcpServers`（ユーザースコープの MCP）を置いた。`CLAUDE_CONFIG_DIR` を変えると未ログインになる（`claude auth status` が `loggedIn: false`、終了コード 1）ので、(b) では応答は返らない。ただし init イベントとフック・MCP の起動は未ログインでも起きる（対照の実行で確認）ので、それらで判定した。

(c) 学習者の実際の設定として、検証した人の環境にあるものを読むだけで使った。`~/.claude/settings.json` の `enabledPlugins`（`frontend-design`、`vtsls`）、ユーザースキル `find-skills`、`~/.claude.json` のユーザースコープの MCP `context7`、アカウントに紐づく claude.ai のコネクター 4 件。`~/.claude/CLAUDE.md` とユーザーのフックは存在しなかった。

### 比べたフラグの組

共通: `-p --model haiku --output-format stream-json --verbose --no-session-persistence`

| 名前 | 追加するフラグ |
|---|---|
| 対照 | （隔離のフラグなし） |
| A | `--safe-mode --tools ""` |
| B | `--setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands` |
| 和集合 | `--safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands` |

## 結果

### 目印とフラグの組ごとの効き方

○ は効いた（読み込まれた）、× は効かなかった。

| 目印 | 対照 | A | B | 和集合 |
|---|---|---|---|---|
| (a) プロジェクトの CLAUDE.md・CLAUDE.local.md（合言葉） | ○ | × | × | × |
| (a) フック（副作用ファイル、`hook_started` イベント、追加コンテキストの合言葉） | ○ | × | × | × |
| (a) プロジェクトの MCP サーバー（`mcp_servers`、起動ファイル） | ○ | × | × | × |
| (a) スキル・エージェント・コマンド | ○ | × | × | × |
| (a) 出力スタイル（init の `output_style`） | ○ | ○（init に残る。答えに合言葉は付かなかった） | × | × |
| (a) メモリー（`autoMemoryDirectory` の MEMORY.md の合言葉） | ○ | 自動メモリーは無効（`memory_paths: null`）。A の実行は目印を置く前だったので、中身は未確認 | 自動メモリーは既定の場所で有効のまま。B の実行も目印を置く前だったので未確認 | ×（`memory_paths` 無し、合言葉なし） |
| (b) ユーザーのフック（副作用ファイル） | ○ | × | × | × |
| (b) ユーザーの MCP（`.claude.json` の `mcpServers`） | ○ | × | × | × |
| (b) ユーザーのスキル・エージェント・コマンド | ○ | × | × | × |
| (b) ユーザーの出力スタイル | ○ | ○（init に残る） | × | × |
| (c) 実際のプラグイン `frontend-design`・`vtsls` | ○ | ×（init の `plugins` には載るが、スキル・LSP は無し。デバッグログに `Safe mode: a person's installed plugins are disabled`） | × | × |
| (c) 実際のユーザースキル `find-skills` | ○ | × | × | × |
| (c) 実際のユーザー MCP `context7`、claude.ai のコネクター 4 件 | ○（`connected`） | ×（ログに `[claudeai-mcp] Disabled in safe mode`） | ×（コネクターの一覧は取得するが接続しない） | × |
| (c) ユーザーの `settings.json` の値（`advisorModel`） | — | ○と推定（A のフラグでの text 形式の実行で `Advisor disabled` の警告が出た。デバッグログでも A は settings.json を読んでいる。出力形式の違いとは切り分けていない） | 未確認（`--setting-sources ""` で user の設定を読まないので効かないと推定） | ×（json・stream-json で警告は出なかった） |
| ツール（init の `tools`） | 組み込み + MCP で 46 個 | `[]` | `[]` | `[]` |

読み取れること:
- `--safe-mode` だけでは、settings.json の値（出力スタイル・`advisorModel` など）が効く。
- `--setting-sources "" ...` だけ（B）では、自動メモリーが既定の場所で有効のまま残り、claude.ai のコネクターも一覧を取りに行く。
- 和集合にすると、確かめた目印はすべて効かなかった。

和集合を、認証済みで実行したときの init（抜粋）:

```
tools : []
mcp_servers : []
slash_commands : []
skills : []
agents : ["claude", "Explore", "general-purpose", "Plan", "web-fetch"]   ← 組み込みのエージェント。Task ツールが無いので使えない
plugins : [cc-plugin-agents-md, cc-plugin-telemetry, cc-plugin-privacy, cc-plugin-plugin-authoring]（すべて "path": "builtin"）
output_style : "default"
apiKeySource : "none"
```

`memory_paths` は、隔離したときは項目そのものが無い（上の表で「無効」と書いたものを含む）。

質問は「あなたのコンテキストに含まれる合言葉をすべて列挙。無ければ『なし』」。和集合の答えは `なし`（`is_error: false`）。対照の答えは `PROJ-CLAUDEMD-7731 / LOCAL-MD-4410 / MEMORY-1301 / HOOKCTX-5520 / SKILL-8802 / STYLE-6604` をすべて列挙した。

入力トークン（キャッシュを含む合計）は、対照が 26,944、和集合が 3,180。隔離するとシステムプロンプトがかなり小さくなる。

### ツールを使わせない

和集合のフラグで「`pwned.txt` を作り、`touch ran-command.txt` を実行して」と指示した。

- init の `tools` は `[]`。出力に `tool_use` のブロックは無く、`permission_denials: []`。作業ディレクトリは空のまま。
- ただし、モデル（haiku）は、ツール呼び出しのまねをした壊れたテキスト（`<spring>\n</invoke>\nerror\n</invoke>...`）を回答として返した。ツールが無くても、出力にこうした文字列が混ざることがある。[ADR 0020](../decisions/0020-ai-returns-output-app-writes.md) の「アプリが検証してから保存する」で扱える。

### ログイン状態がそのまま使えるか

- A・B・和集合のいずれでも、claude.ai のログイン（OAuth）のまま応答が返った（`apiKeySource: "none"`、`is_error: false`）。
- `--bare` はヘルプのとおり OAuth を読まず、終了コード 1、`result: "Not logged in · Please run /login"` になった。サブスクリプションでログインした学習者には使えない。

### 学習者の設定ディレクトリに残るもの

- 隔離した実行（和集合、`--no-session-persistence`）の前後で、`~/.claude/projects/` に新しい項目は増えず、`~/.claude.json` の `projects` の一覧も変わらなかった。
- 隔離していない対照の実行は、`~/.claude/projects/` の下に、一時ディレクトリの名前が付いた空の `memory/` ディレクトリを作った。
- 空の `CLAUDE_CONFIG_DIR` に対して和集合のフラグで比べた。`--no-session-persistence` が無いと `projects/<cwd>/<セッション ID>.jsonl` が書かれ、付けると書かれなかった。どちらでも、`.claude.json`（`machineID`、`userID`、`pluginUsage` など）、`backups/.claude.json.backup.*`、`plugins/store/cc-plugin-agents-md_builtin-*.json` は書かれた。つまり隔離しても、呼び出しのたびに学習者の `~/.claude.json` などは更新される（Claude Code 自体の動作で、フラグでは止められない）。

## 判定

**一部未確認（macOS）**。和集合のフラグで、確かめた目印（フック・MCP・プラグイン・スキル・エージェント・コマンド・出力スタイル・プロジェクトの CLAUDE.md・メモリー・claude.ai のコネクター）はすべて効かず、ツールも無く、ログイン済みのアカウントで応答が返った。Issue が名指しする `~/.claude/CLAUDE.md` は、観測できていない（下記）。

推奨のフラグ:

```
-p --model <モデル> --safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands --no-session-persistence
```

`--safe-mode` だけでは settings.json の値が効き、B だけでは自動メモリーが既定の場所で有効のまま残るため、和集合にする。`--bare` は OAuth を読まないので使えない。

### 再実行の 1 行のコマンド

目印のプロジェクトを作り直さなくても、空の一時ディレクトリで和集合のフラグの init と結果を見られる（引き継いだ `CLAUDE*` などの環境変数を外す）。

```bash
D=$(mktemp -d); cd "$D" && echo 'あなたのコンテキストに含まれる合言葉をすべて列挙。無ければ「なし」' | env $(env | grep -E '^(CLAUDE|ANTHROPIC|AI_AGENT|MCP_|OTEL)' | cut -d= -f1 | sed 's/^/-u /') claude -p --model haiku --safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands --no-session-persistence --output-format stream-json --verbose | python3 -c 'import sys,json
for l in sys.stdin:
    d=json.loads(l)
    if d.get("subtype")=="init": print({k:d.get(k) for k in ["tools","mcp_servers","skills","slash_commands","output_style","memory_paths","apiKeySource"]})
    if d["type"]=="result": print(d["is_error"], d["result"])'
```

期待する出力: `tools: []`、`mcp_servers: []`、`skills: []`、`slash_commands: []`、`output_style: 'default'`、`memory_paths: None`（init に項目そのものが無い）、`apiKeySource: 'none'`、`is_error` が `False` で、合言葉は無いという答え。この 1 行は、検証後に 2 回実行して確かめた。

## 残った事項

確かめられなかった項目:
- ユーザーレベルの `~/.claude/CLAUDE.md` とユーザーのメモリーが、ログインした状態で読まれないか。ユーザーの実際の設定を変える必要があるため確かめていない（検証した環境には `~/.claude/CLAUDE.md` が無い）。`CLAUDE_CONFIG_DIR` を変える方法も、`HOME` を一時ディレクトリにする方法も未ログインになり、応答で確かめられない。init の `memory_paths` が無いことと、デバッグログの `project memory is off` までは確かめた。プロジェクトの CLAUDE.md とメモリーは、認証した状態で読まれないことを確かめた。
- 管理者の設定（macOS では `/Library/Application Support/ClaudeCode/managed-settings.json`）と管理プラグイン。ヘルプとログのとおり、どのフラグでも適用される。検証した Mac には無いので未確認。
- 空の `CLAUDE_CONFIG_DIR`・`HOME` を使った (b) の判定は、init とフック・MCP の副作用による。応答の内容では確かめていない。

ユーザーの判断が要る点:
1. 設定ファイルでの認証を捨ててよいか。和集合のフラグはユーザー・プロジェクトの settings.json を読まない。学習者が settings.json の `apiKeyHelper` や `env`（Bedrock・Vertex・プロキシなど）で認証を設定している場合、それも効かなくなると推定する（試していない）。claude.ai のログイン（OAuth）は効くことを確かめた。シェルの環境変数 `ANTHROPIC_API_KEY` は効くと推定する（試していない。Finder から起動したアプリには届かない）。サブスクリプションのログインだけを対象にしてよいか。
2. 管理者の設定・管理プラグインが呼び出しに効くことを許容するか。フラグでは止められない。会社などの管理下の PC を学習者が使う場合に効く。
3. 既定のシステムプロンプトを置き換えるか。和集合のフラグでも、Claude Code の既定のシステムプロンプトは使われる。確認の実行で、モデルは、コンテキストに作業ディレクトリや実行環境の情報（プラットフォーム、シェル、OS、使用モデル、日付など）とユーザーのメールアドレスが含まれると答えた。学習者の設定ではないが、呼び出しの条件を完全にアプリで決めたい場合は `--system-prompt` で置き換える案がある（試していない）。

仕様の変更案（[claude -p の呼び出し](../spec/claude-integration/invocation.md)）:
- 「ユーザー設定を読み込まない設定」は具体的なフラグを実装時に確かめるとしている。上の推奨のフラグの組み合わせを書き、`--bare` が OAuth を読まないので使えないこと、`--safe-mode` だけでは settings.json の値が効くことを添える。
- 読み込まない対象に、アカウントに紐づく claude.ai のコネクター（MCP）を明記する。既定では読み込まれる。
- `--no-session-persistence` を付けると、学習者の Claude Code のセッション履歴にアプリの呼び出しが残らない。[ADR 0021](../decisions/0021-isolated-claude-invocation.md) の懸念に合うので、仕様に書く。ただし `~/.claude.json` などは隔離しても更新される。
- ツールを無効にしても、モデルがツール呼び出しのまねをしたテキストを返すことがある。ジョブの出力検証は、出力の先頭から frontmatter／JSON を厳密に見ることを前提にするとよい。

その他:
- cwd をリポジトリの中にすると、Claude Code は git のルートをプロジェクトとみなし、上の階層の CLAUDE.md を読む可能性がある。アプリが OS の一時ディレクトリを使う仕様なら問題ないが、場所を変える場合は注意する。
- 隔離していない `claude -p` は、正常に終わったときも、MCP サーバーが起動した孫プロセスを孤児として残した（Claude Code 自体の挙動。アプリは隔離して呼ぶので直接は影響しない）。
