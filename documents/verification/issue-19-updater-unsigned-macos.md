---
type: Verification Report
title: "署名なしの macOS アプリの自動更新の技術検証"
description: "署名なしの macOS アプリで Tauri の自動更新が動くかと、GitHub Actions でのビルドと Releases への公開、Windows インストーラーの WebView2 導入処理を Issue #19 で検証した結果（App Translocation されると更新が失敗すること、quarantine 属性の値ごとの結果、private リポジトリの Releases を認証なしで取れないこと、ad-hoc 署名の有無による違い）を記す。"
---

- 日付: 2026-11-21（検証は 2026-11-20 から 2026-11-21）
- Issue: [#19](https://github.com/Progrust/progrust-app/issues/19)
- 確かめた OS: macOS 27.2.1（Apple Silicon、arm64）。対応環境ではない Windows は、GitHub Actions でのビルドとインストーラーの中身の確認だけを行った
- 関連仕様: [公開範囲と配布](../spec/non-functional/distribution.md)、[対応環境](../spec/non-functional/environments.md)、[非機能要件の未決事項](../spec/non-functional/open-questions.md)、[ADR 0037](../decisions/0037-bundle-content-and-auto-update.md)、[ADR 0038](../decisions/0038-no-os-code-signing-for-now.md)

## 環境

| 項目 | 値 |
|---|---|
| OS | macOS 27.2.1、Apple Silicon（arm64） |
| Rust | rustc 1.99.1、cargo 1.99.1（ターゲットは aarch64-apple-darwin だけ。x86_64 は入れていない） |
| Node / npm | v25.9.0 / 11.12.1 |
| Xcode | Command Line Tools だけ（Apple clang 21.0.0） |
| Tauri | CLI 2.12.6、`tauri` 2.12.7、`tauri-plugin-updater` 2.14.1、`tauri-plugin-process` 2.4.0 |
| GitHub Actions | `tauri-apps/tauri-action@v1`、`macos-latest`、`windows-latest`、`actions/checkout@v7`、`actions/setup-node@v7`、`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`、`actions/upload-artifact@v7` |
| リポジトリ | Progrust/progrust-app（private） |

## 方法

### 試作アプリ

最小の Tauri v2 アプリを、検証用のブランチに置いて作った。フロントエンドはビルドなしの HTML 1 枚（`withGlobalTauri: true`）。

- 起動引数 `--auto` を付けると、起動時に更新を確かめ、あればダウンロード・インストールし、フロントエンドから tauri-plugin-process の `relaunch()` で再起動する。更新が無ければ結果を書いて終了する。
- `--endpoint=URL` で latest.json の URL を差し替える。`--log=PATH` に、時刻・pid・自分の版・実行ファイルのパス・結果を 1 行ずつ追記する。
- 再起動は `open -n -g <bundle> --args ...` で行われる（tauri 2.12.7 の `restart_macos_app`）。起動引数は引き継がれるので、新しい版も同じログに書く。

```rust
// 更新の確認とインストール（抜粋）
let mut b = app.updater_builder();
if let Some(ep) = &o.endpoint { b = b.endpoints(vec![ep.parse()?])?; }
let updater = b.build()?;
match updater.check().await? {
    None => write_log(.., "RESULT no update available"),
    Some(update) => {
        update.download_and_install(|chunk, _| { .. }, || { .. }).await?;
        write_log(.., &format!("RESULT installed {}", update.version));
    }
}
```

```js
// フロントエンド（抜粋）
const r = await invoke('spike_update');
if (r.startsWith('installed:')) await window.__TAURI__.process.relaunch();
else await window.__TAURI__.process.exit(0);
```

`tauri.conf.json` の決め手の部分（updater の設定、署名、WebView2）:

```jsonc
"bundle": {
  "targets": "all",
  "createUpdaterArtifacts": true,
  "windows": { "webviewInstallMode": { "type": "downloadBootstrapper", "silent": true } }
  // 2 回目のビルドだけ "macOS": { "signingIdentity": "-" }（ad-hoc 署名）を足した
},
"plugins": { "updater": {
  "pubkey": "<tauri signer generate で作った公開鍵>",
  "endpoints": ["https://github.com/Progrust/progrust-app/releases/latest/download/latest.json"],
  "dangerousInsecureTransportProtocol": true   // 検証で http://127.0.0.1 から配るため。本番では使わない
}}
```

### 更新用の鍵

`npx tauri signer generate --ci -w <鍵のファイル> -p <ランダムなパスワード>` で使い捨ての鍵を作った。秘密鍵とそのパスワードは、GitHub Actions の secret に入れ、ワークフローで環境変数 `TAURI_SIGNING_PRIVATE_KEY` と `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` に渡した。鍵はリポジトリに commit していない。

### ワークフロー

検証用ブランチへの push で動く GitHub Actions のワークフローを書いた。

1. `create-releases`（ubuntu）: `gh release create spike-updater-v<版> --prerelease` で、2 つの版の pre-release を先に作る（macOS と Windows のジョブが同時に同じ Release を作ろうとして競合しないように）。
2. `build`（matrix: 版 × OS）: 版を tauri.conf.json に書き込み、`npm ci`、`tauri-apps/tauri-action@v1` でビルドして Release に上げる。
   - macOS: `--target universal-apple-darwin`（Rust の targets に aarch64・x86_64 を入れる）。ビルド後に `lipo -info`、`codesign -dv`、`codesign --verify --deep --strict`、`spctl -a -vv` をログに出す。
   - Windows: `updaterJsonPreferNsis: true`。生成された NSIS スクリプトと WiX ソースを workflow artifact に上げ、WebView2 の処理を grep する。

決め手の部分（1 回目の、両 OS をビルドした版）:

```yaml
permissions:
  contents: write
jobs:
  build:
    needs: create-releases
    strategy:
      fail-fast: false
      matrix:
        version: ['0.1.0', '0.1.1']
        platform: [macos-latest, windows-latest]
    runs-on: ${{ matrix.platform }}
    steps:
      # checkout、setup-node、rust-toolchain（macOS は aarch64-apple-darwin,x86_64-apple-darwin）、
      # rust-cache、版を tauri.conf.json に書き込む、npm ci（省略）
      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.<秘密鍵の secret> }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.<パスワードの secret> }}
        with:
          projectPath: spike-updater
          tagName: spike-updater-v__VERSION__
          releaseName: 'spike-updater v__VERSION__ (Issue #19 検証用)'
          prerelease: true
          updaterJsonPreferNsis: true
          args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin --verbose' || '--verbose' }}
```

実行は 2 回。

| 回 | 版 | OS | macOS の署名の設定 |
|---|---|---|---|
| 1 | 0.1.0、0.1.1 | macOS・Windows | なし（既定） |
| 2 | 0.2.0、0.2.1 | macOS だけ | `bundle.macOS.signingIdentity: "-"`（ad-hoc 署名） |

### 更新の試し方（手元の Mac）

リポジトリが private なので、Releases の資産は認証なしで取れない（後述）。そこで、`gh release download` で落とした CI のビルドを、手元の HTTP サーバー（`python3 -m http.server 18719 --bind 127.0.0.1`）で配った。latest.json は CI が作ったものの `url` だけを `http://127.0.0.1:18719/...` に書き換え、署名（`signature`）はそのまま使った。

古い版のアプリは CI の `.dmg` を `hdiutil attach` して `ditto` でコピーし、`open -n <app> --args --auto --endpoint=... --log=...` で起動した。条件は次のとおり。

- quarantine 属性なし（curl や gh で落としたとき、または `xattr -cr` で外したとき）
- `com.apple.quarantine` を付ける（`xattr -r -w`。バンドルの中のすべてのファイルに付ける）。値は 3 種類。
  - `0083;...;Safari;...`: ブラウザーで落とした直後（未承認）を模したもの
  - `01c1;...`: この Mac の `/Applications` にある、ブラウザーで落として初回起動を通した実際のアプリ（Ghostty、Obsidian など）に付いている値
  - `00c3;...`: `0083` に「承認済み」のビット（0x40）だけを足したもの
- 置き場所: `/Applications/spike-updater.app` と、作業用のディレクトリ

注意: `01c1` は、Developer ID で署名・公証された市販のアプリに付いていた値である。署名なしのアプリで「このまま開く」を通した後に同じ値になるかは確かめていない。したがって `01c1` の結果は「この値のときは App Translocation されず、更新が動く」ことを示すだけで、学習者が実際にその状態になれることは示していない。

手元で aarch64 だけのビルド（署名の設定なし、リンカの ad-hoc 署名のみ）でも同じことを先に試した。

## 結果

### Actions で両 OS 向けにビルドし、Releases に公開できるか: できた

1 回目（全ジョブ成功）の所要時間は、macOS が各 約 3.5 分（aarch64 と x86_64 を順にビルドして lipo）、Windows が各 約 9.5〜12.5 分（cargo build だけで 7 分 36 秒）。2 回目（macOS だけ）も成功した。

| 版 | Release に上がった資産 |
|---|---|
| 0.1.0、0.1.1 | `*_universal.dmg`（8.9 MB）、`*_universal.app.tar.gz`（9.1 MB）と `.sig`、`*_x64-setup.exe`（NSIS、3.1 MB）と `.sig`、`*_x64_en-US.msi`（4.5 MB）と `.sig`、`latest.json` |
| 0.2.0、0.2.1 | macOS の資産と `latest.json` だけ |

macOS のジョブのログの抜粋:

```
Running [tauri_cli] Command `cargo build --bins --features tauri/custom-protocol --release --target aarch64-apple-darwin`
Running [tauri_cli] Command `cargo build --bins --features tauri/custom-protocol --release --target x86_64-apple-darwin`
Running [tauri_cli] Command `lipo -create -output .../target/universal-apple-darwin/release/spike-updater ...`
Bundling [tauri_bundler::bundle::updater_bundle] .../bundle/macos/spike-updater.app.tar.gz
Finished [tauri_cli::bundle] 1 updater signature at: ...
latest.json successfully uploaded.
Architectures in the fat file: macos/spike-updater.app/Contents/MacOS/spike-updater are: x86_64 arm64
```

- latest.json の `platforms` には `darwin-aarch64`、`darwin-x86_64`、`darwin-universal` と、それぞれの `-app` 付きの 6 つ（すべて同じユニバーサルの `.app.tar.gz` を指す）と、`windows-x86_64`（`updaterJsonPreferNsis: true` なので NSIS）、`windows-x86_64-nsis`、`windows-x86_64-msi` が入った。
- macOS と Windows のジョブが、同じ Release の latest.json に順に書き足す。今回は macOS が先に終わったので両方の OS が残ったが、同時に終わると片方が消える可能性がある（未確認）。
- tauri-action v1 の latest.json の `url` は `https://api.github.com/repos/Progrust/progrust-app/releases/assets/<id>` の形で、ブラウザー用の URL ではない。updater はダウンロード時に `Accept: application/octet-stream` を付ける（tauri-plugin-updater 2.14.1 の `updater.rs`）ので、public なリポジトリならこの URL で取れる想定（確かめていない）。

### 署名なしの macOS アプリで、更新して再起動できるか: quarantine 属性の値による

| # | ビルド | quarantine | 置き場所 | 結果 |
|---|---|---|---|---|
| ci-A | CI ユニバーサル、署名の設定なし | なし | /Applications | OK: 0.1.0 から 0.1.1 に更新し、relaunch で 0.1.1 が起動（約 1 秒） |
| ci-C | 同上 | `01c1` | /Applications | OK: Gatekeeper のダイアログは出ない |
| ci-B | 同上 | `0083`（未承認） | 作業用のディレクトリ | 止まる: Gatekeeper のダイアログ（人の操作が要る）。プロセスは App Translocation された場所で一時停止したまま |
| ci-D | 同上 | `00c3` | 作業用のディレクトリ | NG: ダイアログは出ずに起動するが、App Translocation された読み取り専用の場所で動くため、更新のインストールが失敗する |
| 手元 | 手元 aarch64、署名の設定なし | なし、`01c1`、`0083`、`00c3` | 作業用のディレクトリ（`01c1` は /Applications も） | CI と同じ |
| 手元 | 手元 aarch64、`signingIdentity: "-"` | なし、`01c1` | 作業用のディレクトリ | OK |

ci-A のログ（パスと pid は短くした）:

```
version=0.1.0 exe=/Applications/spike-updater.app/Contents/MacOS/spike-updater | STARTED args=[…, "--auto", "--endpoint=http://127.0.0.1:18719/ci/latest.json", …]
version=0.1.0 … | update found: 0.1.0 -> 0.1.1 target=darwin url=http://127.0.0.1:18719/ci/spike-updater_0.1.1_universal.app.tar.gz
version=0.1.0 … | RESULT installed 0.1.1 (9081967 bytes)
version=0.1.0 … | js: calling relaunch()
version=0.1.1 exe=/Applications/spike-updater.app/Contents/MacOS/spike-updater | STARTED args=[…同じ引数…]
version=0.1.1 … | RESULT no update available
```

ci-D のログ（App Translocation されたとき）:

```
version=0.1.0 exe=/private/var/folders/…/T/AppTranslocation/EF6D14B0-…/d/spike-updater.app/Contents/MacOS/spike-updater | STARTED …
version=0.1.0 … | update found: 0.1.0 -> 0.1.1 …
version=0.1.0 … | download finished
version=0.1.0 … | ERROR download_and_install: Read-only file system (os error 30) at path "/private/var/folders/…/AppTranslocation/EF6D14B0-…/d/tauri_current_appEEYNBd"
```

- 失敗しても元のアプリ（0.1.0、quarantine 属性付き）はそのまま残り、壊れない。
- 原因: updater（2.14.1 の macOS の `install_inner`）は、実行中のバンドルのパスの隣か一時ディレクトリに作業用のディレクトリを作り、rename でバンドルを入れ替える。App Translocation 中は、そのパスが読み取り専用でマウントされた複製なので、作業用のディレクトリを作れない。この失敗は「権限が無い」（PermissionDenied）ではないので、管理者のパスワードを求める処理にも進まない。
- App Translocation が起きる条件（一般に知られていること。今回は確かめていない）: quarantine 属性が付いたアプリを、Finder で移動しないまま開いたとき（例: ダウンロードフォルダーで zip を展開して開いた、dmg の中から直接開いた）。
- 今回観測したこと: App Translocation は quarantine 属性の値で決まり、置き場所では決まらなかった。`0083` と `00c3` では作業用のディレクトリでも `/Applications` でも起き（`00c3` を /Applications に置いても `Read-only file system` で失敗）、`01c1` では起きなかった。Finder でドラッグして `/Applications` に入れたときに属性がどうなるかは、Finder の操作が要るので確かめていない。つまり「/Applications に入れれば更新できる」とは、今回の結果からは言えない。

ci-B（未承認の quarantine）の Gatekeeper の記録（`log show --predicate 'process == "syspolicyd"'`）:

```
GK evaluateScanResult: 0, PST: (…), (team: (null)), (id: spike_updater), (bundle_id: dev.progrust.spikeupdater), 1, 0, 1, 0, 7, 7, 0
Prompt shown (8, 0), waiting for response: PST: (…), (id: spike_updater), (bundle_id: dev.progrust.spikeupdater)
```

- ダイアログが出て、プロセスは App Translocation されたパスで止まったまま（ログは 1 行も書かれない）。ダイアログの文面は、画面の収録の権限が無く読めなかった。`Prompt shown` の最初の数字が、ビルドの署名の状態で違った。
  - CI ユニバーサル、署名の設定なし（x86_64 のスライスは無署名）: `(8, 0)`
  - 手元 aarch64、署名の設定なし（リンカの ad-hoc 署名だけ）: `(1, 0)`
  - CI ユニバーサルを手元で `codesign --force --deep -s -` で ad-hoc 署名し直したもの: `(6, 0)`
- 数字の意味を説明した公開の資料は見つからなかった。Apple Silicon で無署名・壊れた署名のアプリをダウンロードすると「壊れているため開けません」と出て、「プライバシーとセキュリティ」の「このまま開く」が使えない例が知られている。Tauri の文書（distribute/sign/macos の「Ad-Hoc Signing」）も、Apple Silicon ではインターネットから入れたアプリすべてにコード署名が要るので ad-hoc 署名が役に立つが、プライバシーとセキュリティで許可する操作は無くならない、と書いている。どのダイアログが出て、「このまま開く」で通せるかは、人が画面で確かめる必要がある（未確認）。

署名の状態:

| ビルド | `codesign -dv` | `codesign --verify --deep --strict` | `spctl -a -vv` |
|---|---|---|---|
| CI ユニバーサル、署名の設定なし（更新後も同じ） | `flags=0x20002(adhoc,linker-signed)`、`Signature=adhoc`、`Sealed Resources=none` | `code object is not signed at all` | `rejected`（`source=no usable signature`） |
| 手元 aarch64、署名の設定なし | `flags=0x20002(adhoc,linker-signed)`、`Sealed Resources=none` | `code has no resources but signature indicates they must be present` | 同じエラー |
| 手元 aarch64、`signingIdentity: "-"`（更新後も同じ） | `flags=0x10002(adhoc,runtime)`、`Sealed Resources version=2` | OK | `rejected` |

- 更新後のアプリに quarantine 属性は付かない（OK のすべてのケースで `xattr -l` に `com.apple.quarantine` が無く、`com.apple.provenance` だけ）。updater は reqwest でダウンロードし、アプリの Info.plist に `LSFileQuarantineEnabled` も無いため。したがって更新後の版を起動するときに Gatekeeper のダイアログは出ない。
- 署名の状態は、更新の前後で変わらない（CI が作ったバンドルがそのまま入るため）。
- `spctl` はどの場合も `rejected`。公証していないので当然で、更新の動作には影響しなかった。

2 回目のビルド（CI ユニバーサル、`bundle.macOS.signingIdentity: "-"`）のログ:

```
Signing [tauri_bundler::bundle::macos::sign] with identity "-"
Signing .../spike-updater.app/Contents/MacOS/spike-updater
Signing .../spike-updater.app
Format=app bundle with Mach-O universal (x86_64 arm64)
CodeDirectory v=20500 size=27706 flags=0x10002(adhoc,runtime) hashes=859+3 location=embedded
Signature=adhoc
Sealed Resources version=2 rules=13 files=1
macos/spike-updater.app: valid on disk
macos/spike-updater.app: satisfies its Designated Requirement
macos/spike-updater.app: rejected        ← spctl（公証していないため）
```

| # | 更新 | quarantine | 置き場所 | 結果 |
|---|---|---|---|---|
| ci2-A | 0.2.0 から 0.2.1（ad-hoc） | なし | /Applications | OK。更新後も `codesign --verify --deep --strict` が通る |
| ci2-C | 0.2.0 から 0.2.1（ad-hoc） | `01c1` | /Applications | OK。更新後に quarantine 属性は無い |
| ci2-D | 0.2.0 から 0.2.1（ad-hoc） | `00c3` | 作業用のディレクトリ | NG: App Translocation で `Read-only file system (os error 30)` |
| ci2-B | 0.2.0（ad-hoc） | `0083` | 作業用のディレクトリ | 止まる: `Prompt shown (6, 0)` |
| ci2-G | 0.2.0 から 0.2.1（ad-hoc） | `00c3` | /Applications | NG: /Applications に置いても App Translocation され、`Read-only file system (os error 30)` |
| ci-mig | 0.1.0（署名の設定なし）から 0.2.1（ad-hoc） | なし | /Applications | OK。署名の状態が変わる更新も通る（updater は OS のコード署名を見ず、更新用の鍵の署名だけを確かめる） |

時間（参考値。1 回だけの測定で、他の作業と同時に動いていた）: 更新の確認からインストール完了まで約 0.12 秒（手元の HTTP サーバー、9.1 MB）、`relaunch()` の呼び出しから新しい版の起動まで約 1 秒。

最低 OS: Info.plist の `LSMinimumSystemVersion` は `10.13`、arm64 のスライスの `minos` は 11.0。macOS 13 は範囲内だが、macOS 13 の実機では確かめていない。updater の macOS 版は `renamex_np(RENAME_SWAP)`（macOS 10.12 から）で入れ替え、使えなければ rename 2 回に切り替える。macOS 13 から入った「アプリの管理」の保護で止められることは、macOS 27 では起きなかった（/Applications に置いた自分自身の更新で、エラーも許可のダイアログも無し）。

### private リポジトリの Releases を認証なしで取れるか: 取れない

`curl -sI`（認証なし）の結果:

```
https://github.com/Progrust/progrust-app/releases/download/spike-updater-v0.1.1/latest.json                  → HTTP/2 404
https://github.com/Progrust/progrust-app/releases/latest/download/latest.json                                → HTTP/2 404
https://github.com/Progrust/progrust-app/releases/download/spike-updater-v0.1.1/…_universal.app.tar.gz      → HTTP/2 404
https://api.github.com/repos/Progrust/progrust-app/releases/assets/<id>（latest.json の url）                → HTTP/2 404
```

- 学習者のアプリからは、このままでは更新を確かめられない。検証では手元の HTTP サーバーで配った。
- `releases/latest/download/...` は pre-release を対象にしない。本番で pre-release を使うなら、エンドポイントの決め方も要る。

### Windows のインストーラーに WebView2 ランタイムの導入処理を含められるか: 含められた（インストールは未確認）

設定は `bundle.windows.webviewInstallMode`。Tauri の文書（distribute/windows-installer の「WebView2 Installation Options」）の選択肢は次のとおり。

| type | インターネット接続 | インストーラーの増え | 備考 |
|---|---|---|---|
| `downloadBootstrapper` | 要る | 0 MB | 既定。インストール時にブートストラッパーを落として実行する |
| `embedBootstrapper` | 要る | 約 1.8 MB | ブートストラッパーを同梱 |
| `offlineInstaller` | 要らない | 約 127 MB | ランタイムのインストーラーを同梱 |
| `fixedVersion` | 要らない | 約 180 MB | 固定版のランタイムを同梱 |
| `skip` | — | 0 MB | 導入しない（非推奨） |

試作では `{ "type": "downloadBootstrapper", "silent": true }` を指定した。Windows のジョブで tauri-bundler が生成した NSIS スクリプトと WiX ソースに、導入処理が入っている（ビルドログの grep の抜粋）:

```
installer.nsi:56:!define INSTALLWEBVIEW2MODE "downloadBootstrapper"
installer.nsi:57:!define WEBVIEW2INSTALLERARGS "/silent"
installer.nsi:581:Section WebView2
installer.nsi:584:    ReadRegStr $4 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
installer.nsi:600:        NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
installer.nsi:633:        ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
main.wxs:199:        <Property Id="INSTALLED_WEBVIEW2_VERSION">
main.wxs:205:        <!-- Download webview bootstrapper mode -->
main.wxs:206:        <CustomAction Id='DownloadAndInvokeBootstrapper' Directory="INSTALLDIR" Execute="deferred" ExeCommand='powershell.exe -NoProfile -windowstyle hidden ...
main.wxs:209:                <![CDATA[NOT(REMOVE OR INSTALLED_WEBVIEW2_VERSION)]]>
```

- NSIS: レジストリで WebView2 の有無を調べ、無ければブートストラッパーを落として `/silent /install` で実行する。失敗したらインストールを中止する。
- MSI: WebView2 が無く、アンインストールでもないときに、PowerShell でブートストラッパーを落として実行する CustomAction が入る。
- WiX のビルドで `ICE03: String overflow ... Table: CustomAction, Column: Target, Key(s): DownloadAndInvokeBoot...` の警告が出た（ビルドは成功）。MSI の WebView2 の処理がこの警告の影響を受けるかは、Windows で確かめていない。
- Windows での実際のインストール、更新、再起動は確かめていない。

## 判定

この検証の後、アプリの完成までは配布と更新の仕組みを作らないことにした（[ADR 0091](../decisions/0091-no-distribution-or-auto-update-until-publication.md)）。下の判定と残った事項は、一般公開をする場合に、配布と更新の方法を決めるときの材料になる。

Issue の基準: 古い版から新しい版へ「更新して再起動」で更新できる。

| 対象 | 判定 | 根拠 |
|---|---|---|
| macOS（更新そのもの） | 条件付きで合格 | Actions のユニバーサルビルド（署名の設定なし、ad-hoc 署名の両方）で、0.1.0 から 0.1.1、0.2.0 から 0.2.1、0.1.0 から 0.2.1 の更新と `relaunch()` による再起動ができた。更新後のアプリには quarantine 属性が付かず、再起動で Gatekeeper に止められない。条件は、quarantine 属性が無いか、`01c1`（公証済みアプリの承認済みの値）であること。App Translocation された状態（`0083`・`00c3`）では、置き場所にかかわらず更新が失敗した |
| macOS（ブラウザーで落とした場合の一連の流れ） | 未確認（人の操作が要る） | quarantine 属性付き（未承認）の初回起動は Gatekeeper のダイアログで止まる。どのダイアログが出るか、「このまま開く」で通せるか、通した後に App Translocation されずに更新できる状態になるかは、画面と Finder の操作が要るので確かめていない |

Issue の基準は満たしていない（ブラウザーで落とした場合の一連の流れが未確認）。検証した時点で採用していた [ADR 0037](../decisions/0037-bundle-content-and-auto-update.md)（自動更新で配布する）と [ADR 0038](../decisions/0038-no-os-code-signing-for-now.md)（OS のコード署名を当面行わない）は、updater の仕組みが署名なしでも動いたので、覆す結果ではなかった。ただし、ブラウザーで落とした学習者が更新できる状態になれるかは未確認で、App Translocation を避ける手順が無いと分かった場合は、手順（`xattr -cr` を案内するなど）か、署名なしで配布する方法を見直す必要がある。

## 残った事項

### 確かめられなかった項目

- ad-hoc 署名のビルドを Safari で dmg として落とし、/Applications にドラッグして開いたときに、どのダイアログが出るか、「プライバシーとセキュリティ」の「このまま開く」で通せるか、その後に更新が動くか（人の操作が要る）。
- Finder でドラッグして /Applications に入れたときの quarantine 属性の値。

### ユーザーの判断が要る点

1. 更新ファイルの配布場所。リポジトリが private のままでは、学習者のアプリが latest.json も更新ファイルも取れない。選択肢は次のとおり。
   - このリポジトリを public にする。
   - 配布専用の public リポジトリを作り、Releases をそこに置く（tauri-action の `owner` / `repo` 入力と、そのリポジトリに書ける token を使う。ソースは private のまま）。
   - GitHub 以外の静的ホスティング（GitHub Pages、Cloudflare R2、S3 など）に置く。latest.json の `url` は自分で組み立てる（tauri-action の `uploadUpdaterJson` は使えない）。
   - 参考: updater に認証ヘッダーを付けて private の API から取る方法もあるが、token をアプリに埋め込むことになるので配布には向かない。
2. macOS の ad-hoc 署名（`bundle.macOS.signingIdentity: "-"`）をするか。費用はかからず、Developer ID 署名・公証とは別物。既定のユニバーサルビルドは x86_64 のスライスが無署名で、`codesign --verify` が通らない。ad-hoc 署名にすると通り、Gatekeeper のダイアログの種類も変わる（`Prompt shown (8, 0)` から `(6, 0)`）。どちらでも更新は動いた。「OS のコード署名は行わない」と定める場合に、ad-hoc 署名が含まれるかを決める必要がある。
3. App Translocation への対処。案は次のとおり。
   - 配布ページに「dmg からアプリを『アプリケーション』フォルダーにドラッグしてから開く」と書く。Finder で移動すると App Translocation されない、と一般に言われるが、今回は置き場所が /Applications でも quarantine の値によっては起きたので、人の確認が要る。
   - アプリが起動時に自分のパスが `/AppTranslocation/` の下（または読み取り専用のボリューム）かを調べ、更新の前に「アプリケーションフォルダーに移してください」と案内する。
   - 更新に失敗したときのエラーを表示する。
   - 配布ページで、ターミナルで `xattr -cr /Applications/Progrust.app` を実行する手順を案内する（quarantine 属性を外すと、今回の観測ではダイアログも App Translocation も起きない。ただし学習者にターミナルの操作を求める）。
4. pre-release の扱い。`releases/latest/download/latest.json` は pre-release を指さない。本番で pre-release を使うなら、エンドポイントの決め方が要る。

### 配布と更新の方法を決めるときに踏まえる点

検証した時点の仕様（配布と更新）は、配布物を GitHub Releases に置くこと、OS のコード署名を行わないこと、初回起動時の OS の警告を回避する手順を配布ページに記載することを定めていた。

- 「OS のコード署名は行わない」: Apple Silicon では実行ファイルに少なくとも ad-hoc 署名が要る（リンカが arm64 のスライスに自動で付ける）。ad-hoc 署名を付けるかどうかでダイアログの種類も変わる。「Developer ID 署名・公証、Windows のコード署名証明書は使わない（macOS は ad-hoc 署名だけ行う）」のように、何をしないかを明確にする案がある（判断 2 しだい）。
- 「初回起動時の OS の警告を回避する手順を配布ページに記載する」: 手順は、警告を回避するだけでなく、App Translocation されない状態にするものでなければならない（App Translocation された状態では自動更新が必ず失敗する）。どの手順でそうなるかは、判断 3 と未確認の項目しだい。
- 「配布物は GitHub Releases に置く」: リポジトリが private のままだと学習者が取れない（判断 1）。
- 検証した時点の未決事項「署名なしの macOS アプリで Tauri の自動更新が問題なく動作するか」: 「App Translocation されていなければ動く。初回起動の操作は人の確認が残る」と分かった。
- 仕様にするか判断が要る実装上の注意: 更新が失敗しても元のアプリは壊れない。updater は `Accept: application/octet-stream` で GitHub API の資産の URL から取る。

### 検証中に見つけた別の問題

- 手元で `tauri build` の既定（dmg を含む）を実行すると、dmg を作る `bundle_dmg.sh` が Finder の AppleScript でタイムアウト（`-1712`）して失敗した。Finder の自動操作の許可が無いためと思われる。開発者の手元で dmg を作るときの注意で、`--bundles app` で避けられる。CI では問題なし。
- Windows のビルドで、WiX の警告 `ICE03: String overflow` と NSIS の警告 `LangString "choowHowToInstall" is not set in language table of language English` が出た（どちらもビルドは成功。Tauri 側の生成物）。
