---
type: Verification Report
title: "AI の応答時間と出力の安定性の技術検証"
description: "記録・分析・問題生成・質疑応答の claude -p 呼び出しが制限時間に収まるか、質疑応答で最初の文字が出るまでの時間、生成した問題が検証に通る割合、記録・分析の出力が形式を守るかを Issue #11 で検証した結果（制限時間はすべて収まり、分析は 142 要素で 214.8 秒、検証は 14 問すべてが修正 2 回以内、pattern ラベルは新しい種類で揺れた）と、試作した指示文 6 本を記す。"
---

- 日付: 2026-11-21
- Issue: [#11](https://github.com/Progrust/progrust-app/issues/11)
- 確かめた OS: macOS 27.2.1（Apple Silicon、arm64）だけ。Windows は確かめていない（当面 macOS に集中する方針のため）
- 関連仕様: [claude -p の呼び出し](../spec/claude-integration/invocation.md)、[指示文](../spec/claude-integration/prompts.md)、[モデルの選択](../spec/claude-integration/model-selection.md)、[Claude Code 連携のエラー](../spec/claude-integration/errors.md)、[ジョブの待ち行列](../spec/claude-integration/job-queue.md)、[AI のジョブ](../spec/learning-record-and-analysis/ai-jobs.md)、[学習記録ファイル](../spec/learning-record-and-analysis/record-file.md)、[分析ファイル](../spec/learning-record-and-analysis/analysis-file.md)、[演習生成の AI への入力](../spec/exercise-generation/ai-input.md)、[生成問題の出力と検証](../spec/exercise-generation/output-and-validation.md)、[質疑応答の AI への入力](../spec/qa-chat/ai-input.md)、[質疑応答の回答の方針](../spec/qa-chat/answer-policy.md)、[演習の形式](../spec/content-format/exercise.md)、[提出](../spec/exercise-runner/submit.md)、[ADR 0020](../decisions/0020-ai-returns-output-app-writes.md)、[ADR 0021](../decisions/0021-isolated-claude-invocation.md)、[ADR 0023](../decisions/0023-model-per-purpose.md)、[ADR 0026](../decisions/0026-separate-facts-from-interpretation.md)、[ADR 0028](../decisions/0028-separate-record-and-analysis-sessions.md)、[ADR 0030](../decisions/0030-incremental-analysis-update.md)、[ADR 0034](../decisions/0034-validate-generated-exercises.md)

## 環境

- OS: macOS 27.2.1、arm64。Claude Code 2.1.329（claude.ai のサブスクリプションでログイン）。rustc / cargo 1.99.1。python 3.15.2（PyYAML 6.0.3）。
- モデル: `--model opus` は `claude-opus-5-5`、`--model sonnet` は `claude-sonnet-5-5`（stream-json の `modelUsage` で確認）。
- 測定中の load average（1 分）は 1.3〜3.4。
- 利用上限: 最初の呼び出しで five_hour の利用率 0.44、最後で 0.69。利用上限のエラーは一度も出なかった。
- `claude -p` の呼び出しは合計 40 回（上限 45 回）。記録 5、分析 8、問題生成 14、修正の依頼 1、欠陥を入れた修正の試験 3、質疑応答 8、再実行コマンドの確認 1。

## 方法

### 呼び出し方（共通）

[隔離起動](issue-8-claude-isolated-invocation.md)と[出力の形](issue-9-claude-output-and-errors.md)の検証の結果に従った。

```
claude -p --model <opus|sonnet> --safe-mode --setting-sources "" --strict-mcp-config --tools "" \
  --disable-slash-commands --no-session-persistence --output-format stream-json --verbose \
  [--include-partial-messages]   # 質疑応答だけ
```

- 指示文は標準入力で渡した。cwd は呼び出しごとに OS の一時ディレクトリに作り、終了後に消した。
- 引き継いだ環境変数のうち、正規表現 `^(CLAUDE|ANTHROPIC|AI_AGENT|MCP_|OTEL)` に合うものを取り除いた。`CLAUDE_EFFORT` も外したので、推論の effort は Claude Code の既定のまま。
- 制限時間は仕様どおり（記録・分析 300 秒、問題生成 600 秒、質疑応答 120 秒）で、超えたら kill する形にした。超えた呼び出しは無かった。
- 仕様との違い: ジョブも `--output-format json` ではなく stream-json で呼んだ。最後の `result` 行は json と同じ内容で、`assistant.error` と `rate_limit_event` も記録できる。所要時間の差は 1 秒前後だった。
- ジョブは 1 件ずつ直列に実行した。質疑応答は記録・分析のジョブと並行した。「経過」はプロセスの起動から終了まで。

### 入力の合成

実装前なので、アプリが作る入力を Python で組み立てた。

- 要素マスタは、章とレッスンの一覧の 142 レッスン（章末復習を除く）に 1 要素ずつ。
- 学習者の履歴を乱数で合成した。典型は ch08-03 まで（記録 50 件・要素 44 件）、後半は ch17-02 まで（116 件・101 件）、全範囲は ch23-04 まで（要素 142 件。分析の最悪の場合）。現在の analysis.md は履歴から `base_level` などを計算して作り、大きさは典型 21.6 KB、後半 48.7 KB、全範囲 66.8 KB。
- 試行ログ 3 つは、学習者のコードの各版を実際に `cargo build` して本物のコンパイラの出力と判定結果を入れた。A は ch08-05 の自作の問題（test 方式、実行 29 回・提出 4 回・質問 3 回、生のログ 55.0 KB。ai-jobs.md の規則で圧縮して 33.2 KB）、B は ch08-04（4.2 KB）、C は生成問題（stdout 方式、難易度 3、12.2 KB）。
- 質疑応答用に、ch08-05 の台本（6 シーン・30 文、`code_blocks` 5 件）を仕様の形式で作った。

### 判定と検証（アプリ側）の実装

- 判定は project-assembly.md・run.md・submit.md・limits.md のとおりに実装した（`--offline`、`RUST_BACKTRACE=0`、許可クレートは `rand = "=0.9.1"`）。test 方式は、対象ファイルの末尾に次を足し、`cargo test --offline --no-run` でできた実行ファイルを引数 `progrust_judge::` で起動して結果行を読んだ。

```rust
#[cfg(test)]
#[path = "../progrust_judge_tests.rs"]
mod progrust_judge;
```

- 生成問題の検証は output-and-validation.md の 1〜8 を実装し、AI を呼ぶ前に、正しい問題 3 つと壊した問題 9 つで試して 12 件すべて期待どおりの判定になることを確かめた。記録・分析の検証は ai-jobs.md の「出力の検証」をそのまま実装した。

### 指示文の試作

仕様（prompts.md）の 5 つに、仕様に無い修正の依頼用の 1 つを加えた。全文は asset として置いた。`{{name}}` はアプリが置き換える値。

| 指示文 | 使う場面 | 埋め込む値 |
|---|---|---|
| [record.txt](issue-11-prompts/record.txt) | 記録のジョブ | `record_template`、`lesson_info`、`existing_patterns`、`error_candidates`、`facts`、`attempt_log`（圧縮済み） |
| [analysis.txt](issue-11-prompts/analysis.txt) | 分析のジョブ | `analysis_template`、`current_analysis`、`record`、`aggregates`、`element_master` |
| [generate.txt](issue-11-prompts/generate.txt) | 問題生成 | `settings`、`learned_elements`、`weak_areas`、`recent_generated`、`exercise_rules`、`allowed_crates`、`output_format` |
| [generate-fix.txt](issue-11-prompts/generate-fix.txt)（仕様に無い） | 検証に失敗した生成問題の修正の依頼 | `original_prompt`、`previous_output`、`validation_errors` |
| [chat-video.txt](issue-11-prompts/chat-video.txt) | 動画視聴中の質疑応答 | `lesson`、`script`、`ai_note`、`code_blocks`、`current_sentence`、`learned_elements`、`history`、`question` |
| [chat-explanation.txt](issue-11-prompts/chat-explanation.txt) | 解説表示中の質疑応答 | `lesson`、`problem`、`explanation`、`solution`、`learner_code`、`history`、`question` |

`generate.txt` の `{{output_format}}` に入れた文（出力する JSON の形、各フィールドの説明、判定の仕組み）は [generate-output-format.txt](issue-11-prompts/generate-output-format.txt) に置いた。

### 生の出力の抜粋

質疑応答の最初の `text_delta`（行頭はプロセス起動からの秒）:

```
1.249	{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"`"}},…}
```

### 再実行の 1 行のコマンド（最初の文字までの時間）

空の一時ディレクトリで、隔離のフラグと部分メッセージを付け、最初の `text_delta` までの秒数を出す。

```bash
D=$(mktemp -d); cd "$D" && echo 'Rust の String と &str の違いを 3 行で' | env $(env | grep -E '^(CLAUDE|ANTHROPIC|AI_AGENT|MCP_|OTEL)' | cut -d= -f1 | sed 's/^/-u /') python3 -c 'import subprocess,sys,time,json
t=time.time();p=subprocess.Popen(["claude","-p","--model","sonnet","--safe-mode","--setting-sources","","--strict-mcp-config","--tools","","--disable-slash-commands","--no-session-persistence","--output-format","stream-json","--verbose","--include-partial-messages"],stdin=sys.stdin,stdout=subprocess.PIPE)
for l in p.stdout:
    d=json.loads(l)
    if d.get("type")=="stream_event" and d["event"].get("delta",{}).get("type")=="text_delta": print("first text_delta %.2f s"%(time.time()-t)); break
p.kill()'
```

期待する出力は `first text_delta 1.xx s` 程度。検証の後に 1 回実行して `first text_delta 1.15 s`（load average 3.10）だった。

## 結果

### 記録・分析・問題生成が制限時間に収まるか（既定のモデル opus）

収まった。もっとも余裕が小さいのは、要素が多いときの分析。

記録（制限 300 秒）:

| 入力 | 回数 | 経過（秒） | 入力トークン | 出力トークン |
|---|---|---|---|---|
| A（55 KB の試行ログ） | 3 | 62.7、72.6、73.0 | 22,666〜22,668 | 6,529〜7,462 |
| B（小さい） | 1 | 25.2 | 8,305 | 2,379 |
| C（中くらい） | 1 | 41.6 | 10,210 | 4,273 |

大きいログ（A）の中央値は 72.6 秒、最大 73.0 秒で、制限の 1/4 程度。

分析（制限 300 秒）:

| 学習者（要素数、analysis.md） | 回数 | 経過（秒） | 入力トークン | 出力トークン |
|---|---|---|---|---|
| 典型（45、21.6 KB） | 4 | 78.5、86.7、97.5、99.7 | 22,787〜25,759 | 10,945〜13,421 |
| 後半（101、48.7 KB） | 3（同じ入力） | 157.5、159.9、160.7 | 43,991 | 22,946〜23,591 |
| 全範囲（142、66.8 KB） | 1 | 214.8 | 58,159 | 31,606 |

- 分析は `elements` の全要素（アプリ管理フィールドを含む）を毎回書き直すため、時間は出力の長さ（要素数）でほぼ決まる。出力は 1 秒あたり 140〜150 トークン程度。
- 中央値は、典型 92.1 秒（4 回）、後半 159.9 秒（3 回）、全範囲 214.8 秒（1 回。制限の 72 %）。

問題生成（制限 600 秒、1 回の呼び出しごと）:

14 本（g01〜g14）の内訳: 学習者は典型 8・後半 6、難易度は 1〜5、方式は stdout 7・test 7、構成は single 8・multi 6（最大 6 ファイル、`src/color/hex.rs` のような階層を含む）、`rand` を使うものが 1 本。経過は 68.5〜195.0 秒、出力は 7,709〜23,728 トークンで、難易度が高く multi のものほど長かった（難易度 5 の g09 は 187.7 秒・23,241 トークン、難易度 1 の g02 は 78.7 秒・8,293 トークン）。

- 1 回目の呼び出しの中央値は 110.1 秒、最大は 195.0 秒（g12、制限の 1/3 程度）。
- 検証（模範解答と初期コードのビルドと全ケースの実行）は 0.9〜2.9 秒（ビルド先が温まっている状態）。ジョブ全体で最長は g12 の 196.0 秒。

### 質疑応答で最初の文字が表示されるまでの時間（既定のモデル sonnet）

プロセスの起動から最初の `text_delta` までは、中央値 1.32 秒（0.92〜4.31 秒、8 回）。仕様に目安は無いので数字だけを記す。

| 場面 | 履歴 | 最初の text_delta（秒） |
|---|---|---|
| 動画（v1〜v3） | なし | 1.25、1.73、1.38 |
| 動画（v4） | 10 往復（9.7 KB） | 3.03（最初の thinking は 1.50） |
| 解説表示中（e1〜e3） | なし・なし・1 往復 | 0.92、0.97、0.96 |
| 解説表示中（e4） | 10 往復（7.5 KB） | 4.31（最初の thinking は 1.54） |

全体は 5.4〜10.1 秒、出力は 453〜1,042 トークン。`system/init` は 0.21〜0.51 秒で届いた。

- 履歴が長い 2 回（v4・e4）だけ、モデルが先に思考を出し、その間は `text_delta` が来なかった。思考の間も `thinking_delta` と `system/thinking_tokens` の行は届くので、アプリは「考えています」のような表示を出せる。
- 回答の方針を目視で確認した。v2「この演習の答えを教えて」は完成コードを出さず考え方だけを答えた。v3 の行番号で指す質問は、渡した `code_blocks` の行番号を正しく踏まえた。v4 の「さっきの例を詳しく」は再生中の文の直前の例を説明した。

### 生成した問題が検証に通る割合、修正 2 回以内で収まるか

| 項目 | 結果 |
|---|---|
| 1 回目の出力で 8 項目すべてに通った | 13 / 14（93 %） |
| 修正 1 回で通った | 1 / 14（g11） |
| 修正 2 回でも通らなかった | 0 / 14 |
| 修正 2 回以内で通った | 14 / 14 |
| JSON の解析 | 14 件と修正 4 件すべて厳密な解析で通った。コードフェンス・前置きは一度も付かなかった |
| `claude` の呼び出しの失敗 | 0 |

- g11 の 1 回目の失敗は検証 5（見出し）。test 方式で、関数の仕様を `## 出力` ではなく `## 問題` の下の `###` に書いた（`見出しが無い: ['problem_md: ## 出力']`）。失敗の内容を渡した修正の依頼 1 回で直った。
- 自然な失敗が 1 件だけだったので、合格した出力にわざと欠陥を入れて修正の依頼の経路を確かめた。3 件とも修正 1 回（29〜47 秒、出力 3,800〜6,600 トークン）で通った。

- 入れた欠陥は、期待する出力の 1 件を逆順にする（検証 7 の失敗）、模範解答に型の合わない行を足す（E0308、検証 7）、初期コードを模範解答と同じにする（検証 8）の 3 つ。修正はそれぞれ、期待する出力を戻す、その行だけを消す、初期コードを未実装に戻す、で直った。

検証の 8 項目に入っていない観察:

- 未学習の構文: 要素 ID の検査（検証 2）は 14 / 14 で通ったが、コードには要素 ID に挙がらない構文が出ることがある。g14（典型の学習者、難易度 5）は、未学習のモジュール（ch10）を使う 2 ファイルの問題にした。`mod card;`・`pub` を「おまじない」と注記して初期コードに用意し、学習者が書く必要は無かった。標準入力の読み込み（ch09-05 が未習）も、stdout 方式 6 問すべてで「おまじない」として用意されていた。
- 判定方式: 典型の学習者の 9 問のうち stdout 方式が 6 問、test 方式が 3 問（うち 2 問はメッセージで指示）。curriculum/exercise-constraints.md の「ch03〜ch08 では test 方式を中心にする」は、生成の入力に含まれない。
- 初期コードがコンパイルエラー: g08・g12 は、検証 8（正解にならないこと）には通った。g08 は、学習者が書くメソッドが初期コードに無く、提出すると隠しテストの側で `error[E0599]: no method named messages_by_level found for struct Log<'a>` が 6 件出る。g12 は、初期コードそのものが `error[E0594]` でビルドできない（トレイト境界の誤りを直す課題で、意図的と読める）。
- 模範解答の空行を除く行数は、難易度 1 で 35 行、2 で 28〜73 行、3 で 38〜69 行、4 で 57〜150 行、5 で 112〜160 行。

### 記録・分析の出力が仕様の形式を守るか

守った。記録 5 / 5、分析 8 / 8 が ai-jobs.md の「出力の検証」にすべて通った。リトライが要る失敗も、アプリによる補正も 0 件。

- 記録の検証: `---` で始まる、frontmatter が YAML として解析できる、`outcome`・`severity` が定義済みの値、`element` が要素マスタにある、本文の必須 7 セクションがある、など。分析の検証: `level` が `base_level ± 1` かつ 1〜4、`focus_recommendation` が要素マスタにある、`recommended_difficulty` が 1〜5、必須セクションがある、など。
- アプリ管理フィールド（`base_level` 〜 `review_due` の 11 項目）は、8 回すべてで集計結果と突き合わせた。8 回で計 8,469 フィールド、不一致 0、欠けた要素・余分な要素も 0。

同じ入力 A での記録 3 回の pattern のラベル（`+` の左が `&str`（E0369）の躓きの例）は、`str_lhs_in_plus`・`str_left_operand`・`str_on_lhs_of_add` と、3 回とも違った。区切り文字の付けすぎ（`separator_fencepost` / `separator_placement`）、空の名簿の扱い（`empty_slice_unhandled` / `empty_slice_not_handled`）、`names[0]` からのムーブ（E0508、`move_out_of_slice_index` / `move_out_of_index`）も揺れた。

- `outcome`（heavy_struggle）、躓きの数（6）、`severity` は 3 回とも同じ。既存のラベルがある躓き（`use_after_move`・`use_after_move_into_fn`）は 3 回とも再利用した。
- 既存のラベルが無い新しい種類の躓きでは、同じ入力でも呼び出しごとにラベルが変わった（4 種類中 4 種類）。同じ E0382 を付ける要素（string.concat か ownership.move）も揺れ、`resolved` の判断も 1 回だけ違った。
- 分析の同じ入力 3 回（ana-C-1〜3）: `recommended_difficulty` は 3 回とも 2、`level` の調整は 3 回とも 0 要素。`focus_recommendation` は先頭の `borrow.rules` が同じで、順序と 5 件目が揺れた。

## 判定

基準は「制限時間に収まり、生成した問題の多くが修正 2 回以内で検証に通る（割合の目安は検証時に決める）」。

- 制限時間: 満たした。記録の最大 73.0 秒（制限 300 秒）、分析の最大 214.8 秒（142 要素、制限 300 秒）、問題生成の最大 195.0 秒（制限 600 秒）。時間切れは 0 回。
- 修正 2 回以内で通る割合: 14 / 14（1 回目で 13 / 14）。割合の目安は決まっていないので、ユーザーの判断を待つ。
- 質疑応答の最初の文字は中央値 1.32 秒、記録・分析の形式の検証は 13 / 13 で通った。
- 総合: 測った項目はすべて通った。割合の目安が決まっていないため、Issue は開いたままにする。既定のモデル（ADR 0023）、制限時間、修正の回数（ADR 0034）を見直す必要は、この結果からは無い。
- 測定は 1 台・1 晩・サブスクリプションの 1 アカウントで、利用率 0.44〜0.7 の時間帯に行った。サーバーの混み具合による出力の速さの違いは測れていない。

## 残った事項

確かめられなかった項目:

- Windows は確かめていない。
- 利用上限のエラーは出なかった。[Issue #9](issue-9-claude-output-and-errors.md) の利用上限の未確認の項目は残る。
- 分析の再構築（10 件の記録をまとめて渡す）の所要時間は試していない。入力は長くなるが、出力の長さは同じと見込む。
- 合成した履歴には章末復習の記録が無く（要素の合成で、章末復習の行を読み落とした）、`base_level` 4 の扱いは確かめていない。時間と形式の測定には影響しない。

ユーザーの判断が要る点:

1. 「多くが修正 2 回以内で通る」の割合の目安。結果は 14 / 14（1 回目で 13 / 14）。例は 90 % 以上。14 件と少なく、1 件の差で 7 ポイント動く。
2. 分析の制限時間 300 秒の余裕。142 要素の学習者で 214.8 秒（制限の 72 %）で、出力の速さが 3 割落ちると超える。選択肢は (a) そのままにする、(b) 分析の出力からアプリ管理フィールドを除き、AI には `level` と `level_reason` だけを出させてアプリが合成する（出力が短くなり時間が大きく減ると見込むが、試していない）、(c) 分析の制限時間を延ばす。
3. pattern ラベルの揺れ。`recurring_patterns` と `base_level` の規則 2（同じ `pattern` が 2 件以上）はラベルの一致に頼る。指示文にラベルの付け方の規則を足すか、アプリ側で近いラベルを寄せるか、揺れを許すか。
4. ch10 より前の学習者への複数ファイルの問題。難易度 4〜5 の目安は「複数ファイルにしてもよい」だが、モジュールは ch10 で学ぶ。g14 のように `mod`・`pub` を「おまじない」として用意させるのを許すか、multi を ch10 以降に限るか。要素 ID の検査では、未学習の構文の有無は分からない。

仕様の変更案（documents/ は直していない）:

1. 修正の依頼の指示文が prompts.md の表に無い。`generate-fix.txt` のような置き場所と入力の項目（最初の依頼・前回の出力・検証の結果）を足す。
2. ジョブも stream-json で呼ぶと、`assistant.error` と `rate_limit_event` が取れる（所要時間・結果の取り出しに問題は無かった）。
3. ai-jobs.md の試行ログの圧縮で、イベントを省いたあとの差分の基準（試作は「残したイベントのうち直前のもの」）と、最初の `run` の扱い（試作は全文）が書かれていない。
4. 分析の時間の大半は、アプリ管理フィールドを含む全要素の書き写しの出力。判断 2 の (b) の案。
5. 生成の入力に判定方式の制約（ch03〜ch08 では test 方式を中心にする、HashMap の走査順、rand は test 方式で）が含まれない。足すかどうか。
6. test 方式の `## 出力` の中身（関数のシグネチャ）を、生成の指示文で明示する（g11 の失敗の原因）。
7. 初期コードがビルドできない問題が 2 件あった（g08・g12）。初期コードの方針（シグネチャと `todo!()` を必ず用意する、コンパイルエラーを直す課題も許す）を、指示文か検証で決める。
