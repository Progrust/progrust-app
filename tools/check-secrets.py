#!/usr/bin/env python3
"""秘密の情報（API キー、トークン、秘密鍵など）が、これから push するものに入っていないかを、gitleaks で確かめる。

使い方（標準ライブラリと同じ名前のファイルを読み込まないように、`-I` を付けて実行する）:
    python3 -I tools/check-secrets.py                  # push する前の確認（npm run secrets:check）
    python3 -I tools/check-secrets.py --all            # すべての参照から届くコミットと、作業ツリーの変更
    python3 -I tools/check-secrets.py --check-version  # gitleaks の版と、検出できることだけを確かめる
    --repo PATH    確かめるリポジトリ（既定は、このスクリプトがあるリポジトリ）
    --base SHA     push 先にすでにあるコミット（何度でも）。指定すると、--remotes の代わりに、これらを基点にする
    --config PATH  gitleaks の設定（既定は、このスクリプトの 1 つ上のディレクトリの .gitleaks.toml）

確かめるもの（documents/spec/architecture/development-environment.md「秘密の情報の検査」）:
    1. コミット: HEAD から届き、どのリモートの追跡ブランチからも届かないコミット（まだ push していないコミット）。
       --base を指定したときは、そのコミットから届かないコミット。--all のときは、すべての参照から届くコミット。
       コミットの中身（メッセージ・作者など。保存されているとおりのバイト）と、差分の足した行と、
       足した・写した・名前を変えたファイルの名前を確かめる。マージコミットは、親ごとの差分を確かめる。
       ほかのローカルのブランチにだけあるコミットは、--all でなければ見ない。ブランチ・タグの名前は見ない。
    2. ステージした変更（インデックスと HEAD の差分で足した行）
    3. 作業ツリーの変更（インデックスの写しに `git add --all` した差分で足した行）。
       git に入っていない新しいファイルを含み、.gitignore で無視するもの（node_modules/、work/ など）は含まない。

確かめる中身は、このスクリプトが git で取り出し（失敗したら止まる）、gitleaks の stdin に渡す。
バイナリと扱うファイル（.gitattributes の -diff・binary を含む）も、テキストとして差分を取る。
毎回、確かめる前に、その場で作ったにせの値が同じ設定で見つかることを確かめる（自己点検。SELF_CHECK_RULES の規則だけ）。
git は、システムとグローバルの設定、置き換えの参照（refs/replace）、grafts を使わずに実行し、出力の形を変える
リポジトリの設定は GIT_CONFIG で決める。リポジトリの設定（.git/config）のほかのものは読む。

終了コード: 0 は見つからなかった、1 は見つかった、2 は確かめられなかった（gitleaks が無い・版が違う・
git や gitleaks の失敗・自己点検の失敗）。見つかった値は、出力に出さない。

標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import secrets
import shutil
import string
import subprocess
import sys
import tempfile
from typing import Dict, List, Optional, Tuple

# 固定する gitleaks の版。変えるときは、同じ変更で、development-environment.md の表と、
# .github/workflows/ci.yml の GITLEAKS_VERSION・GITLEAKS_SHA256 を直す。
GITLEAKS_VERSION = "8.30.1"

TOOL_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

EXIT_OK = 0
EXIT_LEAKS = 1
EXIT_SETUP = 2
# gitleaks が、見つけたときに返す終了コード。gitleaks は、設定の誤りなどの失敗でも 1 で終わるので、分ける。
GITLEAKS_FOUND = 99

# git の出力を、リポジトリの設定（.git/config。システムとグローバルの設定は読まない）に左右されないようにする。
# 出力の形を変える設定（色、textconv、外部の diff、パスの書き方、マージの差分の形、メッセージの文字コード、フック）を、
# ここで決める。
GIT_CONFIG = [
    "-c", "core.quotePath=false",
    "-c", "core.hooksPath=" + os.devnull,
    # GIT_GRAFT_FILE を指すと、git が、grafts は廃止予定だという案内を毎回出す。案内だけを止める
    "-c", "advice.graftFileDeprecated=false",
    "-c", "color.ui=never",
    "-c", "diff.noprefix=false",
    "-c", "diff.mnemonicPrefix=false",
    "-c", "diff.relative=false",
    "-c", "log.showRoot=true",
    "-c", "log.showSignature=false",
    "-c", "log.diffMerges=separate",
    "-c", "i18n.logOutputEncoding=UTF-8",
]
DIFF_OPTIONS = ["-p", "--raw", "-U0", "--text", "--no-textconv", "--no-ext-diff", "--no-color", "--no-relative"]
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
# 自己点検で、にせの値が見つかることを確かめる、gitleaks の既定の規則
SELF_CHECK_RULES = ("github-pat", "aws-access-token", "private-key")
HUNK_RE = re.compile(rb"^@@ -\d+(?:,(\d+))? \+\d+(?:,(\d+))? @@")


class Failure(Exception):
    """確かめられなかった（終了コード 2）。"""


def safe_path(roots: List[str]) -> str:
    """PATH から、リポジトリの中と node_modules の中を除く（npm が node_modules/.bin を先頭に足すため）。"""
    real_roots = [os.path.realpath(root) for root in roots]
    kept = []
    for entry in os.environ.get("PATH", "").split(os.pathsep):
        if not entry or not os.path.isabs(entry):
            continue
        real = os.path.realpath(entry)
        if "node_modules" in real.split(os.sep):
            continue
        if any(real == root or real.startswith(root + os.sep) for root in real_roots):
            continue
        kept.append(entry)
    return os.pathsep.join(kept)


def base_env(path: str, empty_dir: str) -> Dict[str, str]:
    """git と gitleaks に渡す環境。git と gitleaks の振る舞いを変える環境変数を外す。

    システムとグローバルの設定（XDG の場所の設定・除外・属性を含む）を読まず、置き換えの参照（refs/replace）と
    grafts を使わない。
    """
    env = {k: v for k, v in os.environ.items() if not k.startswith(("GIT_", "GITLEAKS_"))}
    env.update(
        {
            "PATH": path,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "XDG_CONFIG_HOME": empty_dir,
            "GIT_NO_REPLACE_OBJECTS": "1",
            "GIT_GRAFT_FILE": os.devnull,
            "GIT_TERMINAL_PROMPT": "0",
            "GIT_PAGER": "cat",
            "LC_ALL": "C",
        }
    )
    return env


class Checker:
    def __init__(self, repo: str, config: str) -> None:
        self.repo = os.path.realpath(repo)
        self.config = os.path.realpath(config)
        self.path = safe_path([self.repo, TOOL_ROOT])
        self.tmp = tempfile.mkdtemp(prefix="check-secrets-")
        self.empty_dir = os.path.join(self.tmp, "empty")
        os.mkdir(self.empty_dir)
        self.env = base_env(self.path, self.empty_dir)
        self.git_cmd = shutil.which("git", path=self.path)
        self.gitleaks = shutil.which("gitleaks", path=self.path)

    def close(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    # ---- 道具 ----

    def git(self, args: List[str], env: Optional[Dict[str, str]] = None, stdin: Optional[bytes] = None) -> bytes:
        if self.git_cmd is None:
            raise Failure("git が見つかりません。")
        result = subprocess.run(
            [self.git_cmd, "-C", self.repo] + GIT_CONFIG + args,
            input=stdin, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env or self.env,
        )
        if result.returncode != 0:
            message = result.stderr.decode("utf-8", "replace").strip()
            raise Failure(f"`git {' '.join(args[:2])}` が終了コード {result.returncode} で失敗しました: {message}")
        return result.stdout

    def check_version(self) -> None:
        how = (
            f"gitleaks {GITLEAKS_VERSION} を入れてください（documents/spec/architecture/development-environment.md"
            "「開発に要るもの」）。"
        )
        if self.gitleaks is None:
            raise Failure(f"gitleaks が見つかりません（リポジトリの中と node_modules の中は探しません）。{how}")
        result = subprocess.run([self.gitleaks, "version"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=self.env)
        version = result.stdout.decode("utf-8", "replace").strip().lstrip("v")
        if result.returncode != 0 or version != GITLEAKS_VERSION:
            raise Failure(
                f"gitleaks の版が {version or '不明'} で、固定した版 {GITLEAKS_VERSION} と違います（使われる gitleaks は "
                f"{self.gitleaks}）。{how}版を上げるときは、tools/check-secrets.py の GITLEAKS_VERSION と、ci.yml と、"
                "仕様の表を同じ変更で直します。"
            )
        if not os.path.isfile(self.config):
            raise Failure(f"gitleaks の設定 {self.config} がありません。")

    def scan(self, data: bytes) -> List[Dict[str, object]]:
        """data を gitleaks の stdin で確かめ、見つかったもの（値は伏せてある）を返す。"""
        report = os.path.join(self.tmp, "report.json")
        if os.path.exists(report):
            os.remove(report)
        argv = [
            self.gitleaks, "stdin",
            "--config", self.config,
            "--gitleaks-ignore-path", self.empty_dir,
            "--ignore-gitleaks-allow",
            "--redact",
            "--no-banner",
            "--no-color",
            "--exit-code", str(GITLEAKS_FOUND),
            "--report-format", "json",
            "--report-path", report,
        ]
        result = subprocess.run(argv, input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=self.env, cwd=self.empty_dir)
        if result.returncode not in (0, GITLEAKS_FOUND):
            message = (result.stdout + result.stderr).decode("utf-8", "replace").strip()
            raise Failure(f"gitleaks が終了コード {result.returncode} で失敗しました:\n{message}")
        try:
            with open(report, encoding="utf-8") as handle:
                findings = json.load(handle)
        except (OSError, ValueError) as error:
            raise Failure(f"gitleaks の結果を読めません: {error}")
        if not isinstance(findings, list):
            raise Failure("gitleaks の結果の形が違います。")
        if (result.returncode == GITLEAKS_FOUND) != bool(findings):
            raise Failure("gitleaks の終了コードと結果が合いません。")
        return findings

    def self_check(self) -> None:
        """その場で作ったにせの値が、同じ設定で、それぞれの規則で見つかることを確かめる。値はファイルに書かない。

        確かめるのは SELF_CHECK_RULES の規則だけで、設定でほかの規則だけを外したときは、ここでは気づかない。
        """
        alnum = string.ascii_letters + string.digits
        upper = string.ascii_uppercase + "234567"
        b64 = string.ascii_letters + string.digits + "+/"

        def rand(chars: str, n: int) -> str:
            return "".join(secrets.choice(chars) for _ in range(n))

        def distinct(chars: str, n: int) -> str:
            # どの文字も 1 回だけ使う。gitleaks の aws-access-token の規則は、文字のばらつき（エントロピー）が
            # 小さい値を検出しないので、同じ文字が重なった乱数だと、まれに、正しい設定でも見つからない
            return "".join(secrets.SystemRandom().sample(chars, n))

        # 規則の形の文字列が、このファイル自体に続けて現れないように、分けて組み立てる
        key_kind = "RSA " + "PRIV" + "ATE KEY"
        body = "\n".join(rand(b64, 64) for _ in range(4))
        samples = [
            f'github = "ghp_{rand(alnum, 36)}"',
            f'aws = "AK{"IA"}{distinct(upper, 16)}"',
            f"-----BEGIN {key_kind}-----\n{body}\n-----END {key_kind}-----",
        ]
        findings = self.scan(("\n\n".join(samples) + "\n").encode("ascii"))
        found = {str(f.get("RuleID")) for f in findings}
        missing = [rule for rule in SELF_CHECK_RULES if rule not in found]
        if missing:
            raise Failure(
                "自己点検で、にせの値が見つかりませんでした（規則 " + "、".join(missing) + "）。gitleaks の設定"
                f"（{self.config}）か、gitleaks そのものがおかしいので、確かめられません。"
            )

    # ---- 確かめる中身を取り出す ----

    def commits_text(self, scope: List[str]) -> Tuple[List[bytes], List[str]]:
        """scope のコミットの、コミットの中身（メッセージ・作者など）と、差分の足した行と、足したファイルの名前。"""
        lines: List[bytes] = []
        places: List[str] = []
        shas = self.git(["rev-list"] + scope + ["--"]).decode("ascii").split()
        if not shas:
            return lines, places
        # コミットの中身は、保存されているとおりのバイトで取る（push されるのは、このバイトである）。
        # encoding のヘッダがあれば、UTF-8 に直したものも確かめる。
        raw = self.git(["cat-file", "--batch"], stdin=("\n".join(shas) + "\n").encode("ascii"))
        offset = 0
        for sha in shas:
            header_end = raw.index(b"\n", offset)
            header = raw[offset:header_end].split()
            if len(header) != 3 or header[1] != b"commit" or header[0].decode("ascii") != sha:
                raise Failure(f"コミット {sha[:12]} を読めません。")
            size = int(header[2])
            body = raw[header_end + 1:header_end + 1 + size]
            offset = header_end + 1 + size + 1
            texts = [body]
            head, _, message = body.partition(b"\n\n")
            for field in head.split(b"\n"):
                if field.startswith(b"encoding "):
                    name = field[len(b"encoding "):].decode("ascii", "replace").strip()
                    try:
                        texts.append(message.decode(name, "replace").encode("utf-8"))
                    except LookupError:
                        pass
            for text in texts:
                for line in text.split(b"\n"):
                    lines.append(line)
                    places.append(f"コミット {sha[:12]} の中身（メッセージ・作者など）")
        token = secrets.token_hex(8)
        marker = ("\x1e" + token + "S ").encode("ascii")
        out = self.git(
            ["log", "--diff-merges=separate", "--full-history", "--no-walk=unsorted"] + DIFF_OPTIONS
            + [f"--format=%x1e{token}S %H", "--stdin", "--"],
            stdin=("\n".join(shas) + "\n").encode("ascii"),
        )
        commit = ""
        diff_lines: List[bytes] = []
        for line in out.split(b"\n"):
            if line.startswith(marker):
                self.add_diff(diff_lines, f"コミット {commit[:12]}", lines, places)
                diff_lines = []
                commit = line[len(marker):].decode("ascii", "replace")
            else:
                diff_lines.append(line)
        self.add_diff(diff_lines, f"コミット {commit[:12]}", lines, places)
        return lines, places

    def diff_text(self, label: str, env: Optional[Dict[str, str]] = None) -> Tuple[List[bytes], List[str]]:
        out = self.git(["diff", "--cached"] + DIFF_OPTIONS + ["HEAD", "--"], env=env)
        lines: List[bytes] = []
        places: List[str] = []
        self.add_diff(out.split(b"\n"), label, lines, places)
        return lines, places

    @staticmethod
    def add_diff(diff: List[bytes], label: str, lines: List[bytes], places: List[str]) -> None:
        """差分の足した行だけを、先頭の + を外して lines に足す。hunk の行数を数えて、中身と見出しを分ける。"""
        path = "?"
        old = new = 0
        for line in diff:
            if old > 0 or new > 0:
                if line.startswith(b"+"):
                    new -= 1
                    lines.append(line[1:])
                    places.append(f"{label} の {path}")
                elif line.startswith(b"-"):
                    old -= 1
                elif line.startswith(b" "):
                    old -= 1
                    new -= 1
                elif line.startswith(b"\\"):
                    pass
                else:
                    old = new = 0
                continue
            if line.startswith(b"\\"):
                continue
            if line.startswith(b":") and b"\t" in line:
                # --raw の行。足した・写した・名前を変えたファイルの名前も確かめる
                fields = line.split(b"\t")
                status = fields[0].split(b" ")[-1][:1]
                if status in (b"A", b"C", b"R") and len(fields) >= 2:
                    lines.append(fields[-1])
                    places.append(f"{label} のファイルの名前")
                continue
            match = HUNK_RE.match(line)
            if match:
                old = int(match.group(1) or b"1")
                new = int(match.group(2) or b"1")
            elif line.startswith(b"diff --git "):
                path = line[len(b"diff --git "):].decode("utf-8", "replace")
            elif line.startswith(b"+++ ") and not line.startswith(b"+++ /dev/null"):
                path = line[4:].rstrip(b"\t").decode("utf-8", "replace")
                if path.startswith("b/"):
                    path = path[2:]

    def worktree_text(self) -> Tuple[List[bytes], List[str]]:
        """インデックスの写しにすべての変更を足して、その写しと HEAD の差分を取る。本物のインデックスは変えない。"""
        index = self.git(["rev-parse", "--path-format=absolute", "--git-path", "index"]).decode("utf-8").strip()
        copy = os.path.join(self.tmp, "index")
        if os.path.isfile(index):
            shutil.copyfile(index, copy)
        env = dict(self.env, GIT_INDEX_FILE=copy)
        self.git(["add", "--all", "--", "."], env=env)
        return self.diff_text("作業ツリーの変更", env=env)

    # ---- 全体 ----

    def run(self, all_refs: bool, bases: List[str]) -> int:
        self.check_version()
        self.self_check()
        try:
            self.git(["rev-parse", "--verify", "--quiet", "HEAD"])
        except Failure:
            raise Failure("HEAD のコミットがありません。")
        parts = []
        if all_refs:
            parts.append(("コミット（すべての参照から届くもの）", self.commits_text(["--all"])))
        elif bases:
            for base in bases:
                if not SHA_RE.match(base):
                    raise Failure(f"--base には、コミットの 40 文字の SHA を渡します: {base}")
                try:
                    self.git(["cat-file", "-e", f"{base}^{{commit}}"])
                except Failure:
                    raise Failure(f"--base のコミット {base} が、リポジトリにありません。")
            parts.append(("push していないコミット", self.commits_text(["HEAD", "--not"] + bases)))
        else:
            parts.append(("push していないコミット", self.commits_text(["HEAD", "--not", "--remotes"])))
        parts.append(("ステージした変更", self.diff_text("ステージした変更")))
        parts.append(("作業ツリーの変更", self.worktree_text()))

        found: List[str] = []
        for title, (lines, places) in parts:
            print(f"== {title}: {len(lines)} 行", flush=True)
            if not lines:
                continue
            for finding in self.scan(b"\n".join(lines) + b"\n"):
                line = finding.get("StartLine")
                place = places[line - 1] if isinstance(line, int) and 0 < line <= len(places) else title
                text = f"{place}（規則 {finding.get('RuleID')}）"
                if text not in found:
                    found.append(text)
        if found:
            print("check-secrets: 秘密の情報が見つかりました（値は伏せてあります）。push しないでください。", file=sys.stderr)
            for text in found:
                print(f"  - {text}", file=sys.stderr)
            print(
                "本物の秘密の情報なら、無効にしてから取り除きます。コミットに入ったものは、後のコミットで消しても履歴に残ります。"
                "誤検出なら、.gitleaks.toml で除きます。",
                file=sys.stderr,
            )
            return EXIT_LEAKS
        print("check-secrets: 秘密の情報は見つかりませんでした。")
        return EXIT_OK


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="python3 -I tools/check-secrets.py",
        description="これから push するもの（push していないコミット、ステージした変更、作業ツリーの変更）を gitleaks で確かめる。",
    )
    parser.add_argument("--all", action="store_true", help="push していないコミットの代わりに、すべての参照から届くコミットを確かめる")
    parser.add_argument("--check-version", action="store_true", help="gitleaks の版と、検出できること（自己点検）だけを確かめる")
    parser.add_argument("--repo", default=TOOL_ROOT, help="確かめるリポジトリ")
    parser.add_argument(
        "--base", action="append", default=[], metavar="SHA",
        help="push 先にすでにあるコミット（何度でも指定できる）。指定すると、手元のリモートの追跡ブランチの代わりに、"
        "これらから届かないコミットを確かめる",
    )
    parser.add_argument("--config", default=None, help="gitleaks の設定（既定は、このスクリプトの 1 つ上のディレクトリの .gitleaks.toml）")
    args = parser.parse_args(argv)
    config = args.config or os.path.join(TOOL_ROOT, ".gitleaks.toml")
    checker = Checker(args.repo, config)
    try:
        if args.check_version:
            checker.check_version()
            checker.self_check()
            print(f"gitleaks {GITLEAKS_VERSION}（自己点検も通った）")
            return EXIT_OK
        if args.all and args.base:
            parser.error("--all と --base は、同時に指定できません。")
        return checker.run(args.all, args.base)
    except Failure as error:
        print(f"check-secrets: 確かめられませんでした: {error}", file=sys.stderr)
        return EXIT_SETUP
    finally:
        checker.close()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
