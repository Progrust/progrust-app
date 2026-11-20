---
type: Verification Report
title: "claude -p の子プロセスとしての扱いの技術検証"
description: "claude -p を子プロセスとして扱えるかを Issue #10 で検証した結果（50 KB の日本語の標準入力、質疑応答とジョブの同時呼び出し、プロセスグループへの kill での子・孫プロセスの停止、Finder から起動したアプリでの claude の検出と認証、npm・Homebrew の探索先が決まっていないこと）を記す。"
---

- 日付: 2026-11-20
- Issue: [#10](https://github.com/Progrust/progrust-app/issues/10)
- 確かめた OS: macOS 27.2.1（Apple Silicon、arm64）
- 関連仕様: [claude -p の呼び出し](../spec/claude-integration/invocation.md)、[ジョブの実行](../spec/claude-integration/job-queue.md)、[外部ツールの検出](../spec/architecture/external-tools.md)、[ADR 0017](../decisions/0017-search-default-tool-locations.md)、[ADR 0021](../decisions/0021-isolated-claude-invocation.md)、[ADR 0024](../decisions/0024-run-jobs-serially.md)

## 環境

| 項目 | 値 |
|---|---|
| OS | macOS 27.2.1、Apple Silicon（arm64） |
| Claude Code | 2.1.329（ネイティブインストーラーで導入。`~/.local/bin/claude` から `~/.local/share/claude/versions/2.1.329` へのリンク） |
| ログイン | claude.ai のサブスクリプション（OAuth）。API キーは使っていない |
| rustc / cargo | 1.99.1 / 1.99.1 |
| node | v25.9.0（mise で導入。npm のグローバル先は `~/.local/share/mise/installs/node/25.9.0/lib/node_modules`） |
| 測定時の負荷 | load average 18〜19（他のエージェントが同時に動いていた。時間の厳密な比較はしていない） |

入れ子で `claude` を実行するため、検証を動かしている環境から引き継いだ `CLAUDE*` などの環境変数を取り除いた。モデルはすべて `haiku`。

## 方法

依存クレートなしの Rust の小さな試作（`std::process` だけ）で確かめた。`claude` は、[隔離起動の技術検証](issue-8-claude-isolated-invocation.md)の推奨のフラグで呼んだ（json、`--no-session-persistence` つき）。要点のコード:

```rust
let mut cmd = Command::new(claude);
cmd.args(&args).current_dir(tmp_dir)
   .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
   .process_group(0);              // 新しいプロセスグループ（pgid = 子の pid）
let mut child = cmd.spawn()?;
let pgid = child.id() as i32;
let mut sin = child.stdin.take().unwrap();
std::thread::spawn(move || { let _ = sin.write_all(&prompt); });   // 書き終えたら drop で閉じる
// stdout / stderr も別スレッドで読み切る
// 制限時間・キャンセル時:
unsafe { kill(-pgid, SIGTERM) };   // グループ全体へ
// 最大 3 秒待ってから、残りがあってもなくても
unsafe { kill(-pgid, SIGKILL) };
```

## 結果

### 長い指示文を標準入力で文字化けせずに渡せるか

渡せる。

- 50,240 バイト（18,230 文字）の日本語（試行ログ風の文と、記号 `①②③「」『』〜ー—…` を含む）の先頭・中ほど・末尾に合言葉 A・B・C を置き、そのまま答えさせた。
- launchd に近い環境で実行した。`env_clear()` のうえで `HOME USER LOGNAME PATH=/usr/bin:/bin:/usr/sbin:/sbin SHELL TMPDIR` だけを渡し、`LANG`・`LC_*` は無し。結果は終了コード 0 で、答え `A=『紅葉の朝に鳴く鹿』 B=『雪解けの川を渡る舟』 C=『夏草に眠る兵どもの夢』` が一字一句一致した。入力トークンの合計は 19,866（指示文が短いときは 3,180）で、全体が届いている。
- Finder から起動したアプリでも同じ結果だった（後述）。

### 質疑応答とジョブを同時に呼び出せるか

呼び出せる。2 つのスレッドから同時に起動した（質疑応答に見立てた stream-json + `--include-partial-messages` と、ジョブに見立てた json）。どちらも終了コード 0、`is_error: false`。開始は +0.00 秒、終了は +3.27 秒（ジョブ）と +3.71 秒（質疑応答）で、時間が重なっていた。2 本より多い同時実行や、長いジョブとの重なりは試していない。

### キャンセル・タイムアウトで子プロセスまでまとめて止められるか

プロセスグループへの kill で止められる。

| 実行 | kill の前のプロセス | kill の方法 | 結果 |
|---|---|---|---|
| 隔離した実行。制限時間 3 秒 | `claude` 自身だけ（子プロセスは無い） | グループへ SIGTERM、続けて SIGKILL | 3.81 秒で終了。終了コード 143（SIGTERM を受けて 128+15）。残ったプロセス無し。標準出力は `system/init` などだけで、`result` は出ない |
| 隔離していない対照（MCP サーバーとその孫プロセスがある）。5.5 秒 | `claude`、`npm exec @upstash/context7-mcp`、`node .../context7-mcp`、`python mcp_server.py`、孫の `sleep`。すべて claude と同じ pgid | グループへ SIGTERM、続けて SIGKILL | 残ったプロセス無し |
| 同じ対照 | 同上 | `claude` の pid だけへ SIGTERM、続けて SIGKILL | claude は MCP サーバーを止めたが、孫の `sleep` が親 1（孤児）で残った |

- claude は MCP サーバーなどの子をプロセスグループから外さない（`setsid` しない）ので、グループへの kill で子・孫までまとめて止められる。
- 隔離した実行では子プロセスが無いので、pid への kill だけでも止まる。それでも、将来の Claude Code の変更に備えてグループへの kill にしておくのがよい。
- 隔離していない実行では、claude が正常に終わったときも MCP サーバーの孫プロセスが孤児で残った。
- ユーザーのキャンセルは、制限時間と同じ関数（同じ kill の処理）を通るので、制限時間のテストで代えた。

### macOS で Finder から起動したとき、`claude` を見つけられ、認証も通るか

通る（ネイティブインストーラーで導入した場合）。

- 試作のバイナリを実行ファイルにした最小の `.app`（`LSUIElement`）を作った。起動したアプリは、環境変数の一覧、`claude` の探索、`claude --version`、`claude auth status`、隔離のフラグでの `claude -p`（上の 50 KB の日本語）を結果ファイルに書く。
- ターミナルから `open` で起動すると、呼び出し元のシェルの環境変数（`PATH`、`LANG`、`CLAUDECODE` など）がアプリにそのまま渡り、Finder からの起動の代わりにならない。そこで次のコマンドで Finder に起動させた。

```
osascript -e 'tell application "Finder" to open (POSIX file "/path/to/Probe.app" as alias)'
```

- Finder から起動したときの環境（全部）:

```
COMMAND_MODE=unix2003
HOME=<ホームディレクトリ>
LOGNAME=<ユーザー名>
OSLogRateLimit=64
PATH=/usr/bin:/bin:/usr/sbin:/sbin
SHELL=/bin/zsh
SSH_AUTH_SOCK=/var/run/com.apple.launchd.…/Listeners
TMPDIR=/var/folders/…/T/
USER=<ユーザー名>
XPC_FLAGS=0x0
XPC_SERVICE_NAME=application.<バンドル ID>.…
__CFBundleIdentifier=<バンドル ID>
```

  `LANG` は無く、`CLAUDE*` も無い。`PATH` に Homebrew（`/opt/homebrew/bin`）も npm も入っていない。
- 探索（仕様の順: `PATH`、`~/.local/bin`、`~/.claude/local`、npm のグローバル先）: `PATH` の 4 つには無く、`~/.local/bin/claude` で見つかった。
- `claude --version` は終了コード 0、`2.1.329 (Claude Code)`。`claude auth status` は `loggedIn: true`、`authMethod: "claude.ai"`。キーチェーンの確認ダイアログは出なかった。
- `claude -p`（隔離のフラグ、json、50 KB の日本語を標準入力で）は、終了コード 0、3.87 秒、合言葉 A・B・C が一字一句一致、標準エラー出力は空。
- npm でインストールした場合、Homebrew でインストールした場合は、この Mac では試していない。

## 判定

**合格（ネイティブインストーラーで導入した `claude`）**。確かめる項目（長い指示文の標準入力、同時の呼び出し、プロセスグループでの停止、Finder から起動したときの検出と認証）はすべて仕様どおりに動いた。

## 残った事項

確かめられなかった項目:
- npm・Homebrew で入れた `claude` の検出と認証。
- 2 本より多い同時実行、長いジョブとの重なり。
- Tauri の shell プラグインの kill を使う場合に、プロセスグループ全体を止められるか（試作は `std::process`）。

ユーザーの判断が要る点:
1. npm・Homebrew で入れた `claude` の探索先。Finder から起動したアプリでは `npm` が `PATH` に無く、`npm root -g` でグローバル先を求められない。node を mise・nvm・volta などで入れていると、グローバル先はバージョンごとのディレクトリになる（この Mac は `~/.local/share/mise/installs/node/25.9.0/...`）。Homebrew の `/opt/homebrew/bin`（Intel では `/usr/local/bin`）も launchd の `PATH` に無い。探す具体的なパス（例: `/opt/homebrew/bin`、`/usr/local/bin`、`~/.npm-global/bin`、nvm・mise・volta の場所）を決める必要がある。

仕様の変更案:
- [外部ツールの検出](../spec/architecture/external-tools.md)「検出」の「npm のグローバルインストール先」は、上のとおり `npm root -g` が使えないので、具体的なパスで決める。
- [claude -p の呼び出し](../spec/claude-integration/invocation.md)「制限時間を超えたらプロセスを終了」に、子・孫プロセスを含めて止めるため、新しいプロセスグループで起動してグループへ kill する、と書く。

その他:
- 検証の手順の注意として、ターミナルから `open` で起動したアプリには呼び出し元の環境変数が渡る。Finder から起動したときの環境を試すときは、Finder から起動する必要がある。
- 隔離していない `claude -p` は、MCP サーバーが起動した孫プロセスを孤児として残した（Claude Code 自体の挙動。アプリは隔離して呼ぶので直接は影響しない）。
