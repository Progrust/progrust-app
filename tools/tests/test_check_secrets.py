#!/usr/bin/env python3
"""tools/check-secrets.py のテスト。本物の git と gitleaks を使う。

実行（CI の secrets のジョブでも実行する。documents/spec/non-functional/testing.md「CI」）:
    python3 -I tools/tests/test_check_secrets.py

一時ディレクトリに git のリポジトリ（と、bare の origin）を作り、秘密の情報の形をしたにせの値を置いて、
スクリプトが見つけること・見つけないこと・確かめられないときに失敗することを確かめる。にせの値は、
実行のたびに作る（このファイルに固定の値を書くと、このファイル自体が検出されるため）。

gitleaks が入っていない、または版が違うときは、テストは失敗する（飛ばさない）。
標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import os
import secrets
import shutil
import stat
import string
import subprocess
import sys
import tempfile
import unittest
from typing import Dict, List, Optional

TOOLS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(TOOLS_DIR, "check-secrets.py")
CONFIG = os.path.join(os.path.dirname(TOOLS_DIR), ".gitleaks.toml")


def fake_token() -> str:
    """gitleaks の既定の規則 github-pat が見つける形の、にせの値。"""
    return "ghp_" + "".join(secrets.choice(string.ascii_letters + string.digits) for _ in range(36))


class CheckSecretsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = os.path.realpath(tempfile.mkdtemp(prefix="check-secrets-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.origin = os.path.join(self.tmp, "origin.git")
        self.repo = os.path.join(self.tmp, "repo")
        self.env = dict(os.environ)
        self.env.update(
            {
                "GIT_CONFIG_GLOBAL": os.devnull,
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_AUTHOR_NAME": "tester", "GIT_AUTHOR_EMAIL": "tester@example.invalid",
                "GIT_COMMITTER_NAME": "tester", "GIT_COMMITTER_EMAIL": "tester@example.invalid",
            }
        )
        self.sh(["git", "init", "--quiet", "--bare", "--initial-branch=main", self.origin], cwd=self.tmp)
        self.sh(["git", "init", "--quiet", "--initial-branch=main", self.repo], cwd=self.tmp)
        self.sh(["git", "remote", "add", "origin", self.origin])
        self.write("a.txt", "最初\n")
        self.commit("最初")
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.token = fake_token()

    # ---- 道具 ----

    def sh(self, argv: List[str], cwd: Optional[str] = None) -> str:
        result = subprocess.run(argv, cwd=cwd or self.repo, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if result.returncode != 0:
            raise AssertionError(f"{argv} が失敗: {result.stderr}")
        return result.stdout

    def write(self, path: str, content, mode: str = "w") -> None:
        full = os.path.join(self.repo, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        if isinstance(content, bytes):
            with open(full, "wb") as handle:
                handle.write(content)
        else:
            with open(full, mode, encoding="utf-8") as handle:
                handle.write(content)

    def commit(self, message: str) -> None:
        self.sh(["git", "add", "-A"])
        self.sh(["git", "commit", "--quiet", "--allow-empty", "-m", message])

    def leak(self) -> str:
        return f'token = "{self.token}"\n'

    def check(self, *args: str, env: Optional[Dict[str, str]] = None, script: str = SCRIPT, config: str = CONFIG) -> subprocess.CompletedProcess:
        argv = [sys.executable, "-I", script, "--repo", self.repo, "--config", config] + list(args)
        result = subprocess.run(argv, cwd=self.repo, env=env or self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.output = result.stdout + result.stderr
        self.assertNotIn(self.token, self.output)  # 見つかった値は出さない
        return result

    def assert_found(self, *places: str, args: List[str] = (), **kwargs) -> None:
        result = self.check(*args, **kwargs)
        self.assertEqual(result.returncode, 1, self.output)
        for place in places:
            self.assertIn(place, self.output)
        self.assertIn("github-pat", self.output)

    def assert_clean(self, *args: str, **kwargs) -> None:
        result = self.check(*args, **kwargs)
        self.assertEqual(result.returncode, 0, self.output)

    def assert_cannot_check(self, text: str, *args: str, **kwargs) -> None:
        result = self.check(*args, **kwargs)
        self.assertEqual(result.returncode, 2, self.output)
        self.assertIn(text, self.output)

    def fake_bin(self, name: str, body: str) -> str:
        directory = os.path.join(self.tmp, f"bin-{name}-{secrets.token_hex(3)}")
        os.makedirs(directory)
        path = os.path.join(directory, name)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(body)
        os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)
        return directory

    # ---- 前提 ----

    def test_gitleaks_of_pinned_version_is_installed(self) -> None:
        result = self.check("--check-version")
        self.assertEqual(result.returncode, 0, self.output)

    # ---- 見つけないもの ----

    def test_nothing_to_check(self) -> None:
        self.assert_clean()
        self.assertIn("push していないコミット: 0 行", self.output)

    def test_ignored_files_are_not_checked(self) -> None:
        self.write(".gitignore", "work/\nnode_modules/\n")
        self.commit(".gitignore")
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.write("work/secret.txt", self.leak())
        self.write("node_modules/x/secret.txt", self.leak())
        self.assert_clean()

    def test_pushed_commits_are_not_checked_without_all(self) -> None:
        self.write("old.txt", self.leak())
        self.commit("前のもの")
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.assert_clean()
        self.assert_found("コミット", "old.txt", args=["--all"])

    # ---- 見つけるもの ----

    def test_untracked_file(self) -> None:
        self.write("new.txt", self.leak())
        self.assert_found("作業ツリーの変更 の new.txt")

    def test_staged_change(self) -> None:
        self.write("a.txt", self.leak(), mode="a")
        self.sh(["git", "add", "a.txt"])
        self.assert_found("ステージした変更 の a.txt")

    def test_staged_then_worktree_reverted(self) -> None:
        self.write("a.txt", self.leak(), mode="a")
        self.sh(["git", "add", "a.txt"])
        self.sh(["git", "restore", "--worktree", "--source=HEAD", "a.txt"])
        self.assert_found("ステージした変更 の a.txt")

    def test_unstaged_change(self) -> None:
        self.write("a.txt", self.leak(), mode="a")
        self.assert_found("作業ツリーの変更 の a.txt")

    def test_added_then_removed_in_unpushed_commits(self) -> None:
        self.write("b.txt", self.leak())
        self.commit("足す")
        os.remove(os.path.join(self.repo, "b.txt"))
        self.commit("消す")
        self.assert_found("コミット", "b.txt")

    def test_without_remote_all_commits_of_head_are_checked(self) -> None:
        self.sh(["git", "remote", "remove", "origin"])
        self.write("b.txt", self.leak())
        self.commit("足す")
        self.assert_found("b.txt")

    def test_names_with_space_and_japanese(self) -> None:
        self.write("ディレクトリ/空白 のある 名前.txt", self.leak())
        self.assert_found("ディレクトリ/空白 のある 名前.txt")

    def test_commit_message(self) -> None:
        self.sh(["git", "commit", "--quiet", "--allow-empty", "-m", "メッセージ\n\n" + self.leak()])
        self.assert_found("の中身（メッセージ・作者など）")

    def test_conflict_resolution_in_merge_commit(self) -> None:
        self.sh(["git", "switch", "--quiet", "-c", "other"])
        self.write("a.txt", "ほかの変更\n")
        self.commit("ほか")
        self.sh(["git", "switch", "--quiet", "main"])
        self.write("a.txt", "こちらの変更\n")
        self.commit("こちら")
        subprocess.run(["git", "merge", "--quiet", "other"], cwd=self.repo, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.write("a.txt", "直した\n" + self.leak())
        self.sh(["git", "add", "a.txt"])
        self.sh(["git", "commit", "--quiet", "--no-edit"])
        self.assert_found("a.txt")

    def test_file_names(self) -> None:
        self.write(f"dir/{self.token}.txt", "中身\n")
        self.commit("名前")
        self.write("plain.txt", "中身\n")
        self.commit("ふつう")
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.sh(["git", "mv", "plain.txt", f"moved-{self.token}.txt"])
        self.commit("名前を変える")
        self.assert_found("のファイルの名前")
        self.assert_found("のファイルの名前", args=["--all"])

    def test_file_name_in_worktree(self) -> None:
        self.write(f"new-{self.token}.txt", "中身\n")
        self.assert_found("作業ツリーの変更 のファイルの名前")

    def test_replaced_objects_are_not_used(self) -> None:
        # 秘密の情報を含む blob を、きれいな blob に置き換えて（refs/replace）からコミットする
        self.write("s.txt", self.leak())
        dirty = self.sh(["git", "hash-object", "-w", "s.txt"]).strip()
        self.write("clean.txt", "きれい\n")
        clean = self.sh(["git", "hash-object", "-w", "clean.txt"]).strip()
        os.remove(os.path.join(self.repo, "clean.txt"))
        self.sh(["git", "replace", dirty, clean])
        self.commit("置き換え")
        self.assert_found("s.txt")

    def test_merges_with_combined_diff_config(self) -> None:
        for value in ("combined", "dense-combined", "first-parent", "off"):
            self.sh(["git", "config", "log.diffMerges", value])
            self.test_conflict_resolution_in_merge_commit() if value == "combined" else self.assert_found("a.txt")

    def test_log_output_encoding_config(self) -> None:
        self.sh(["git", "commit", "--quiet", "--allow-empty", "-m", "メッセージ " + self.leak()])
        for value in ("UTF-16", "UTF-16LE", "UTF-32"):
            self.sh(["git", "config", "i18n.logOutputEncoding", value])
            self.assert_found("の中身（メッセージ・作者など）")

    def test_message_stored_in_utf16_with_encoding_header(self) -> None:
        tree = self.sh(["git", "rev-parse", "HEAD^{tree}"]).strip()
        parent = self.sh(["git", "rev-parse", "HEAD"]).strip()
        head = f"tree {tree}\nparent {parent}\nauthor t <t@e.invalid> 0 +0000\ncommitter t <t@e.invalid> 0 +0000\nencoding UTF-16LE\n\n"
        body = head.encode("ascii") + ("メッセージ " + self.leak()).encode("utf-16-le")
        path = os.path.join(self.tmp, "commit.raw")
        with open(path, "wb") as handle:
            handle.write(body)
        sha = self.sh(["git", "hash-object", "-t", "commit", "-w", "--literally", path]).strip()
        self.sh(["git", "update-ref", "refs/heads/main", sha])
        self.assert_found("の中身（メッセージ・作者など）")

    def test_global_config_is_not_read(self) -> None:
        # グローバルの設定で、git に入っていない .txt をすべて無視させても、確かめる
        home = os.path.join(self.tmp, "home")
        os.makedirs(home)
        excludes = os.path.join(home, "ignore")
        with open(excludes, "w", encoding="utf-8") as handle:
            handle.write("*.txt\n")
        config = os.path.join(home, ".gitconfig")
        with open(config, "w", encoding="utf-8") as handle:
            handle.write(f"[core]\n\texcludesFile = {excludes}\n")
        self.write("h.txt", self.leak())
        env = dict(self.env, HOME=home, GIT_CONFIG_GLOBAL=config)
        self.assert_found("h.txt", env=env)

    def test_base_instead_of_remote_tracking_branches(self) -> None:
        pushed = self.sh(["git", "rev-parse", "HEAD"]).strip()
        self.write("b.txt", self.leak())
        self.commit("足す")
        # 手元の追跡ブランチを、push していないコミットに向けても、--base を基点にすれば見つかる
        self.sh(["git", "update-ref", "refs/remotes/origin/zz", "HEAD"])
        self.assert_clean()
        self.assert_found("b.txt", args=["--base", pushed])
        self.assert_cannot_check("40 文字", "--base", "main")
        self.assert_cannot_check("ありません", "--base", "0" * 40)

    def test_gitleaks_exit_code_and_report_must_agree(self) -> None:
        # 見つけたと報告しながら 0 で終わる、おかしな gitleaks は、確かめられないとして失敗にする
        body = (
            "#!/bin/sh\n"
            'if [ "$1" = version ]; then echo 8.30.1; exit 0; fi\n'
            'while [ $# -gt 0 ]; do if [ "$1" = --report-path ]; then '
            "printf '[{\"RuleID\":\"github-pat\",\"StartLine\":1},{\"RuleID\":\"aws-access-token\",\"StartLine\":1},"
            "{\"RuleID\":\"private-key\",\"StartLine\":1}]' > \"$2\"; fi; shift; done\n"
            "cat > /dev/null\nexit 0\n"
        )
        directory = self.fake_bin("gitleaks", body)
        env = dict(self.env, PATH=directory + os.pathsep + self.env["PATH"])
        self.assert_cannot_check("終了コードと結果が合いません", env=env)

    def test_config_that_drops_one_self_check_rule(self) -> None:
        dropped = os.path.join(self.tmp, "dropped.toml")
        with open(dropped, "w", encoding="utf-8") as handle:
            handle.write('[extend]\nuseDefault = true\ndisabledRules = ["aws-access-token"]\n')
        self.assert_cannot_check("aws-access-token", config=dropped)

    def test_gitattributes_binary_and_no_diff(self) -> None:
        self.write(".gitattributes", "secrets.cfg -diff\nkey.dat binary\n")
        self.write("secrets.cfg", self.leak())
        self.commit("属性")
        self.write("key.dat", self.leak())
        self.assert_found("secrets.cfg", "key.dat")

    def test_plain_text_inside_real_binary(self) -> None:
        self.write("blob.bin", b"\x00\x01\x02binary\x00\n" + self.leak().encode("ascii") + b"\x00\xff\xfe\n")
        self.commit("バイナリ")
        self.assert_found("blob.bin")

    def test_gitleaks_allow_comment_is_ignored(self) -> None:
        self.write("c.txt", f'token = "{self.token}"  # gitleaks:allow\n')
        self.assert_found("c.txt")

    def test_local_git_config_color_and_textconv(self) -> None:
        self.sh(["git", "config", "color.ui", "always"])
        self.sh(["git", "config", "diff.hide.textconv", "true"])
        self.sh(["git", "config", "diff.external", "true"])
        self.write(".gitattributes", "*.cfg diff=hide\n")
        self.write("x.cfg", self.leak())
        self.commit("色と textconv")
        self.assert_found("x.cfg")

    def test_other_branches_only_with_all(self) -> None:
        self.sh(["git", "switch", "--quiet", "-c", "side"])
        self.write("side.txt", self.leak())
        self.commit("横")
        self.sh(["git", "switch", "--quiet", "main"])
        self.assert_clean()
        self.assert_found("side.txt", args=["--all"])

    def test_stdlib_named_file_next_to_script_is_not_imported(self) -> None:
        tools = os.path.join(self.repo, "tools")
        os.makedirs(tools)
        shutil.copyfile(SCRIPT, os.path.join(tools, "check-secrets.py"))
        with open(os.path.join(tools, "tempfile.py"), "w", encoding="utf-8") as handle:
            handle.write("raise SystemExit(0)\n")
        with open(os.path.join(tools, "json.py"), "w", encoding="utf-8") as handle:
            handle.write("raise SystemExit(0)\n")
        self.write("d.txt", self.leak())
        self.assert_found("d.txt", script=os.path.join(tools, "check-secrets.py"))

    def test_gitleaks_in_node_modules_bin_is_not_used(self) -> None:
        shadow = os.path.join(self.repo, "node_modules", ".bin")
        os.makedirs(shadow)
        path = os.path.join(shadow, "gitleaks")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("#!/bin/sh\necho 8.30.1\nexit 0\n")
        os.chmod(path, 0o755)
        self.write("e.txt", self.leak())
        env = dict(self.env, PATH=shadow + os.pathsep + self.env["PATH"])
        self.assert_found("e.txt", env=env)

    # ---- 確かめられないとき ----

    def test_gitleaks_missing(self) -> None:
        git = shutil.which("git")
        directory = os.path.join(self.tmp, "only-git")
        os.makedirs(directory)
        os.symlink(git, os.path.join(directory, "git"))
        self.assert_cannot_check("gitleaks が見つかりません", env=dict(self.env, PATH=directory))

    def test_gitleaks_version_mismatch(self) -> None:
        directory = self.fake_bin("gitleaks", "#!/bin/sh\necho 8.29.0\n")
        env = dict(self.env, PATH=directory + os.pathsep + self.env["PATH"])
        self.assert_cannot_check("固定した版 8.30.1 と違います", env=env)

    def test_fake_gitleaks_that_finds_nothing_fails_self_check(self) -> None:
        body = (
            "#!/bin/sh\n"
            'if [ "$1" = version ]; then echo 8.30.1; exit 0; fi\n'
            'while [ $# -gt 0 ]; do if [ "$1" = --report-path ]; then echo "[]" > "$2"; fi; shift; done\n'
            "cat > /dev/null\nexit 0\n"
        )
        directory = self.fake_bin("gitleaks", body)
        env = dict(self.env, PATH=directory + os.pathsep + self.env["PATH"])
        self.write("f.txt", self.leak())
        self.assert_cannot_check("自己点検", env=env)

    def test_broken_config(self) -> None:
        broken = os.path.join(self.tmp, "broken.toml")
        with open(broken, "w", encoding="utf-8") as handle:
            handle.write("[extend\n")
        self.assert_cannot_check("gitleaks が終了コード", config=broken)

    def test_config_without_default_rules_fails_self_check(self) -> None:
        for text in ("", "[extend]\nuseDefault = false\n"):
            empty = os.path.join(self.tmp, "empty.toml")
            with open(empty, "w", encoding="utf-8") as handle:
                handle.write(text)
            result = self.check(config=empty)
            self.assertEqual(result.returncode, 2, self.output)

    def test_missing_config(self) -> None:
        self.assert_cannot_check("がありません", config=os.path.join(self.tmp, "none.toml"))

    def test_git_failure_is_not_a_pass(self) -> None:
        self.write("g.txt", self.leak())
        self.commit("足す")
        blob = self.sh(["git", "rev-parse", "HEAD:g.txt"]).strip()
        os.remove(os.path.join(self.repo, ".git", "objects", blob[:2], blob[2:]))
        os.remove(os.path.join(self.repo, "g.txt"))
        self.sh(["git", "rm", "--quiet", "--cached", "g.txt"])
        self.assert_cannot_check("git")


if __name__ == "__main__":
    unittest.main()
