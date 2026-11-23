#!/usr/bin/env python3
"""tools/implement-loop.py のテスト。

実行:
    python3 -m unittest tools/tests/test_implement_loop.py

一時ディレクトリに git のリポジトリ（bare の origin と clone）を作り、PATH の先頭に置いた
偽の `gh` と偽の `claude` で、スクリプトを端から端まで動かす。GitHub にも Claude にも
つながない。偽のツールが返す内容は、実行ファイルの隣のファイルで決める。

標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from typing import Any, Dict, List, Optional

TOOLS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(TOOLS_DIR, "implement-loop.py")


def load_module():
    """スクリプトを、テストごとに新しいモジュールとして読み込む（定数を差し替えるため）。"""
    spec = importlib.util.spec_from_file_location("implement_loop", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    saved = sys.dont_write_bytecode
    sys.dont_write_bytecode = True  # tools/ に __pycache__ を作らない
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = saved
    return module


FAKE_GH = r'''#!/usr/bin/env python3
"""偽の gh。状態は隣の gh-state.json に持ち、呼び出しを gh-calls.jsonl に残す。"""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
STATE = os.path.join(HERE, "gh-state.json")
CALLS = os.path.join(HERE, "gh-calls.jsonl")
FLAGS_WITHOUT_VALUE = {"--squash", "--log-failed"}


def main():
    argv = sys.argv[1:]
    stdin = "" if sys.stdin.isatty() else sys.stdin.read()
    with open(CALLS, "a", encoding="utf-8") as handle:
        handle.write(json.dumps({"argv": argv, "stdin": stdin}, ensure_ascii=False) + "\n")
    with open(STATE, encoding="utf-8") as handle:
        state = json.load(handle)

    positional, flags = [], {}
    index = 0
    while index < len(argv):
        arg = argv[index]
        if arg.startswith("--"):
            if arg in FLAGS_WITHOUT_VALUE:
                flags.setdefault(arg, []).append("")
            else:
                index += 1
                flags.setdefault(arg, []).append(argv[index])
        else:
            positional.append(arg)
        index += 1
    command = " ".join(positional[:2])

    def flag(name, default=None):
        return flags[name][-1] if name in flags else default

    def save():
        with open(STATE, "w", encoding="utf-8") as handle:
            json.dump(state, handle, ensure_ascii=False, indent=1)

    def fail(message, code=1):
        sys.stderr.write(message + "\n")
        sys.exit(code)

    if command in state.get("fail", {}):
        fail(state["fail"][command])

    def issue_json(issue, fields):
        full = {
            "number": issue["number"],
            "title": issue["title"],
            "body": issue["body"],
            "state": issue["state"],
            "labels": [{"name": name} for name in issue["labels"]],
            "milestone": {"title": issue["milestone"]} if issue.get("milestone") else None,
        }
        return {key: full[key] for key in fields}

    def get_issue(number):
        issue = state["issues"].get(str(number))
        if issue is None:
            fail(f"GraphQL: Could not resolve to an Issue with the number of {number}.")
        return issue

    if command == "issue list":
        wanted = flag("--state", "open")
        labels = flags.get("--label", [])
        fields = flag("--json").split(",")
        out = []
        for issue in sorted(state["issues"].values(), key=lambda i: -i["number"]):
            if wanted != "all" and issue["state"].lower() != wanted:
                continue
            if any(label not in issue["labels"] for label in labels):
                continue
            out.append(issue_json(issue, fields))
        print(json.dumps(out, ensure_ascii=False))
    elif command == "issue view":
        print(json.dumps(issue_json(get_issue(positional[2]), flag("--json").split(",")), ensure_ascii=False))
    elif command == "issue edit":
        issue = get_issue(positional[2])
        for names in flags.get("--add-label", []):
            for name in names.split(","):
                if name not in issue["labels"]:
                    issue["labels"].append(name)
        for names in flags.get("--remove-label", []):
            for name in names.split(","):
                if name in issue["labels"]:
                    issue["labels"].remove(name)
        if "--body-file" in flags:
            issue["body"] = stdin
        save()
    elif command == "issue comment":
        get_issue(positional[2])["comments"].append(stdin)
        save()
    elif command == "issue close":
        issue = get_issue(positional[2])
        issue["state"] = "CLOSED"
        issue["close_reason"] = flag("--reason", "")
        save()
    elif command == "pr list":
        head = flag("--head")
        out = [{"number": pr["number"]} for pr in state["prs"].values() if pr["head"] == head and pr["state"] == "OPEN"]
        print(json.dumps(out))
    elif command == "pr create":
        number = state["next_pr"]
        state["next_pr"] += 1
        state["prs"][str(number)] = {
            "number": number, "head": flag("--head"), "base": flag("--base"),
            "title": flag("--title"), "body": stdin, "state": "OPEN",
        }
        save()
        print(f"https://github.com/example/repo/pull/{number}")
    elif command == "pr edit":
        pr = state["prs"][positional[2]]
        if "--title" in flags:
            pr["title"] = flag("--title")
        if "--body-file" in flags:
            pr["body"] = stdin
        save()
    elif command == "pr checks":
        pr = state["prs"][positional[2]]
        queue = state.get("checks_queue", [])
        checks = queue.pop(0) if queue else state.get("checks", [])
        save()
        if not checks:
            fail(f"no checks reported on the '{pr['head']}' branch")
        print(json.dumps(checks))
        buckets = {check["bucket"] for check in checks}
        sys.exit(8 if "pending" in buckets else 1 if buckets & {"fail", "cancel"} else 0)
    elif command == "pr merge":
        pr = state["prs"][positional[2]]
        scratch = state["scratch"]
        if not os.path.isdir(scratch):
            subprocess.run(["git", "clone", "--quiet", state["origin"], scratch], check=True)
        def git(*args):
            return subprocess.run(["git", "-C", scratch] + list(args), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        git("fetch", "--quiet", "origin")
        git("checkout", "--quiet", "-B", "main", "origin/main")
        merged = git("merge", "--squash", "origin/" + pr["head"])
        if merged.returncode != 0:
            git("reset", "--hard", "--quiet")
            fail("Pull request is not mergeable: " + merged.stderr)
        message = flag("--subject", pr["title"]) + "\n\n" + flag("--body", "")
        committed = git("commit", "--quiet", "-m", message)
        if committed.returncode != 0:
            fail("commit failed: " + committed.stdout + committed.stderr)
        pushed = git("push", "--quiet", "origin", "main")
        if pushed.returncode != 0:
            fail("push failed: " + pushed.stderr)
        pr["state"] = "MERGED"
        save()
    elif command == "run view":
        print(state.get("run_log", "テストが失敗した行"))
    else:
        fail(f"偽の gh が知らないコマンド: {argv}", 2)


main()
'''

FAKE_CLAUDE = r'''#!/usr/bin/env python3
"""偽の claude。返す内容は隣の claude-script.json で決め、呼び出しを claude-calls.jsonl に残す。"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "claude-script.json")
CALLS = os.path.join(HERE, "claude-calls.jsonl")


def main():
    argv = sys.argv[1:]
    def value(name):
        return argv[argv.index(name) + 1] if name in argv else None
    prompt = value("-p") or ""
    kind = "implement" if prompt.startswith("/implement-task") else "review" if prompt.startswith("/review-task") else "triage" if prompt.startswith("/triage-issues") else "other"
    with open(SCRIPT, encoding="utf-8") as handle:
        script = json.load(handle)
    count = script.get("count", 0) + 1
    script["count"] = count
    settings = value("--settings")
    settings_text = None
    if settings and os.path.isfile(settings):
        with open(settings, encoding="utf-8") as handle:
            settings_text = handle.read()
    with open(CALLS, "a", encoding="utf-8") as handle:
        handle.write(json.dumps({
            "kind": kind, "prompt": prompt, "model": value("--model"), "resume": value("--resume"),
            "settings": settings, "settings_text": settings_text, "cwd": os.getcwd(),
            "permission_mode": value("--permission-mode"), "has_schema": "--json-schema" in argv,
            "output_format": value("--output-format"),
        }, ensure_ascii=False) + "\n")
    queue = script.get(kind, [])
    entry = queue.pop(0) if queue else script.get(kind + "_default")
    with open(SCRIPT, "w", encoding="utf-8") as handle:
        json.dump(script, handle, ensure_ascii=False, indent=1)
    if entry is None:
        sys.stderr.write(f"偽の claude: {kind} の返す内容が残っていない\n")
        sys.exit(3)

    for path, content in (entry.get("files") or {}).items():
        os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(content)
    for path in entry.get("delete") or []:
        if os.path.exists(path):
            os.remove(path)
    if entry.get("commit"):
        # 禁止をすり抜けて、claude がコミットしたことにする
        subprocess.run(["git", "add", "-A"], check=True)
        subprocess.run(["git", "commit", "--quiet", "--allow-empty", "-m", "claude のコミット"], check=True)
    if entry.get("push_main"):
        # ほかの作業で main が進んだことにする
        with tempfile.TemporaryDirectory() as tmp:
            subprocess.run(["git", "clone", "--quiet", script["origin"], tmp], check=True)
            for path, content in entry["push_main"].items():
                with open(os.path.join(tmp, path), "w", encoding="utf-8") as handle:
                    handle.write(content)
            subprocess.run(["git", "-C", tmp, "add", "-A"], check=True)
            subprocess.run(["git", "-C", tmp, "commit", "--quiet", "-m", "ほかの作業"], check=True)
            subprocess.run(["git", "-C", tmp, "push", "--quiet", "origin", "HEAD:main"], check=True)

    if "raw_stdout" in entry:
        sys.stdout.write(entry["raw_stdout"])
        sys.exit(entry.get("exit", 0))
    structured = entry.get("structured_output")
    out = {
        "type": "result",
        "subtype": "success",
        "is_error": bool(entry.get("is_error", False)),
        "result": entry.get("result", json.dumps(structured, ensure_ascii=False) if structured else "ok"),
        "session_id": value("--resume") or f"sess-{count}",
        "permission_denials": entry.get("permission_denials", []),
        "api_error_status": entry.get("api_error_status"),
    }
    if structured is not None:
        out["structured_output"] = structured
    print(json.dumps(out, ensure_ascii=False))
    sys.exit(entry.get("exit", 0))


main()
'''

CHECK_STUB = '''#!/usr/bin/env python3
import os, sys
if os.path.exists("FAIL_DOCS"):
    print("documents/x.md:1: link-target: FAIL_DOCS がある")
    sys.exit(1)
'''


def done(**extra: Any) -> Dict[str, Any]:
    result = {
        "status": "done", "pr_title": "機能を足す", "summary": "機能を足した。",
        "fixed_problems": [], "human_checks": [], "blocked_reason": "", "blocking_issues": [],
    }
    result.update(extra)
    return result


def review_pass(*notes: str) -> Dict[str, Any]:
    return {
        "structured_output": {
            "verdict": "pass", "summary": "問題なし。",
            "findings": [{"severity": "note", "file": "a.txt", "description": note} for note in notes],
        }
    }


def review_fail(*musts: str) -> Dict[str, Any]:
    return {
        "structured_output": {
            "verdict": "fail", "summary": "直す点がある。",
            "findings": [{"severity": "must", "file": "a.txt", "description": must} for must in musts],
        }
    }


PASSING = [{"name": "check", "state": "SUCCESS", "bucket": "pass", "link": "https://github.com/example/repo/actions/runs/1/job/2"}]
FAILING = [{"name": "check", "state": "FAILURE", "bucket": "fail", "link": "https://github.com/example/repo/actions/runs/7/job/8"}]
PENDING = [{"name": "check", "state": "IN_PROGRESS", "bucket": "pending", "link": ""}]


class LoopTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.mkdtemp(prefix="implement-loop-test-")
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.bin = os.path.join(self.tmp, "bin")
        self.origin = os.path.join(self.tmp, "origin.git")
        self.repo = os.path.join(self.tmp, "repo")
        self.worktree = os.path.join(self.tmp, "repo-loop")
        self.logs = os.path.join(self.tmp, "logs")
        os.makedirs(self.bin)

        self.saved_env = dict(os.environ)
        self.addCleanup(self.restore_env)
        os.environ["PATH"] = self.bin + os.pathsep + os.environ.get("PATH", "")
        os.environ.update(
            {
                "GIT_CONFIG_GLOBAL": os.devnull,
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_AUTHOR_NAME": "tester", "GIT_AUTHOR_EMAIL": "tester@example.invalid",
                "GIT_COMMITTER_NAME": "tester", "GIT_COMMITTER_EMAIL": "tester@example.invalid",
            }
        )

        for name, source in (("gh", FAKE_GH), ("claude", FAKE_CLAUDE)):
            path = os.path.join(self.bin, name)
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(source)
            os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)

        self.sh(["git", "init", "--quiet", "--bare", "--initial-branch=main", self.origin], cwd=self.tmp)
        self.sh(["git", "init", "--quiet", "--initial-branch=main", self.repo], cwd=self.tmp)
        self.sh(["git", "remote", "add", "origin", self.origin])
        files = {
            ".claude/loop-settings.json": '{"permissions": {"allow": ["Edit"]}}\n',
            ".claude/loop-review-settings.json": '{"permissions": {"allow": ["Read"]}}\n',
            ".claude/loop-triage-settings.json": '{"permissions": {"allow": ["Bash(gh issue *)"]}}\n',
            ".claude/skills/implement-task/SKILL.md": "実装\n",
            ".claude/skills/review-task/SKILL.md": "レビュー\n",
            ".claude/skills/triage-issues/SKILL.md": "仕分け\n",
            ".github/workflows/ci.yml": "name: ci\n",
            "CLAUDE.md": "決まり\n",
            "tools/check-documents.py": CHECK_STUB,
            "tools/design-system.py": "import sys\nsys.exit(0)\n",
            "shared.txt": "最初\n",
        }
        for path, content in files.items():
            full = os.path.join(self.repo, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as handle:
                handle.write(content)
        self.sh(["git", "add", "-A"])
        self.sh(["git", "commit", "--quiet", "-m", "最初のコミット"])
        self.sh(["git", "push", "--quiet", "origin", "main"])

        self.state: Dict[str, Any] = {
            "issues": {}, "prs": {}, "next_pr": 100, "checks": PASSING, "checks_queue": [],
            "fail": {}, "origin": self.origin, "scratch": os.path.join(self.tmp, "merge-clone"),
        }
        self.script: Dict[str, Any] = {"origin": self.origin, "implement": [], "review": [], "triage": []}

        self.mod = load_module()
        self.mod.CI_INITIAL_WAIT = 0
        self.mod.CI_POLL_SECONDS = 0.01
        self.mod.CI_APPEAR_TIMEOUT = 0.2
        self.mod.CI_TIMEOUT = 5

    def restore_env(self) -> None:
        os.environ.clear()
        os.environ.update(self.saved_env)

    # ---- 道具 ----

    def sh(self, argv: List[str], cwd: Optional[str] = None) -> str:
        result = subprocess.run(argv, cwd=cwd or self.repo, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if result.returncode != 0:
            raise AssertionError(f"{argv} が失敗: {result.stderr}")
        return result.stdout

    def add_issue(self, number: int, labels: List[str], body: str = "- 前提: なし\n", title: Optional[str] = None,
                  milestone: Optional[str] = None, state: str = "OPEN") -> None:
        self.state["issues"][str(number)] = {
            "number": number, "title": title or f"タスク{number}", "body": body, "labels": list(labels),
            "state": state, "milestone": milestone, "comments": [],
        }

    def task(self, number: int, difficulty: str = "小", **kwargs: Any) -> None:
        labels = ["実装タスク", f"難易度:{difficulty}"] + kwargs.pop("extra_labels", [])
        self.add_issue(number, labels, **kwargs)

    def write_fakes(self) -> None:
        with open(os.path.join(self.bin, "gh-state.json"), "w", encoding="utf-8") as handle:
            json.dump(self.state, handle, ensure_ascii=False)
        with open(os.path.join(self.bin, "claude-script.json"), "w", encoding="utf-8") as handle:
            json.dump(self.script, handle, ensure_ascii=False)

    def run_loop(self, *extra: str) -> int:
        self.write_fakes()
        argv = ["--repo", self.repo, "--worktree", self.worktree, "--log-dir", self.logs] + list(extra)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = self.mod.main(argv)
        self.output = out.getvalue()
        with open(os.path.join(self.bin, "gh-state.json"), encoding="utf-8") as handle:
            self.state = json.load(handle)
        return code

    def read_calls(self, name: str) -> List[Dict[str, Any]]:
        path = os.path.join(self.bin, name)
        if not os.path.exists(path):
            return []
        with open(path, encoding="utf-8") as handle:
            return [json.loads(line) for line in handle if line.strip()]

    def claude_calls(self, kind: Optional[str] = None) -> List[Dict[str, Any]]:
        calls = self.read_calls("claude-calls.jsonl")
        return [c for c in calls if kind is None or c["kind"] == kind]

    def gh_calls(self, command: str) -> List[Dict[str, Any]]:
        return [c for c in self.read_calls("gh-calls.jsonl") if " ".join(c["argv"][:2]) == command]

    def issue(self, number: int) -> Dict[str, Any]:
        return self.state["issues"][str(number)]

    def main_file(self, path: str) -> Optional[str]:
        result = subprocess.run(["git", "--git-dir", self.origin, "show", f"main:{path}"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        return result.stdout if result.returncode == 0 else None

    def origin_branches(self) -> List[str]:
        out = self.sh(["git", "--git-dir", self.origin, "for-each-ref", "--format=%(refname:short)", "refs/heads"])
        return out.split()


class PureFunctionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = load_module()

    def test_parse_prerequisites(self) -> None:
        parse = self.mod.parse_prerequisites
        self.assertIsNone(parse("本文だけ"))
        self.assertEqual(parse("x\n- 前提: なし\ny"), [])
        self.assertEqual(parse("- 前提: #12, #13\n"), [12, 13])
        self.assertEqual(parse("- 前提：#7\n"), [7])

    def test_add_prerequisites(self) -> None:
        add = self.mod.add_prerequisites
        self.assertIsNone(add("本文だけ", [5]))
        self.assertEqual(add("a\n- 前提: なし\nb", [5]), "a\n- 前提: #5\nb")
        self.assertEqual(add("- 前提: #3\n", [3, 9]), "- 前提: #3, #9\n")

    def test_human_check_items(self) -> None:
        items = self.mod.human_check_items
        self.assertEqual(items("## 概要\nx\n"), [])
        self.assertEqual(items("## 人が確かめること\n\n- なし\n\n## 次\n- y\n"), [])
        self.assertEqual(items("## 人が確かめること\n\nなし。\n"), [])
        self.assertEqual(items("## 人が確かめること\n\n- [ ] 画面を見る\n- 音を聞く\n\n## 次\n- y\n"), ["画面を見る", "音を聞く"])

    def test_is_protected(self) -> None:
        protected = self.mod.is_protected
        for path in ("CLAUDE.md", ".claude/skills/x/SKILL.md", ".github/workflows/ci.yml", "documents/conventions/issues.md",
                     "tools/implement-loop.py", "tools/okf/validate.py", "tools/tests/test_implement_loop.py"):
            self.assertTrue(protected(path), path)
        for path in ("src/main.ts", "documents/spec/overview.md", "tools/check-content.ts", "docs/CLAUDE.md"):
            self.assertFalse(protected(path), path)

    def test_merge_unique(self) -> None:
        self.assertEqual(self.mod.merge_unique(["a"], ["b", "a", " ", "b"]), ["a", "b"])


class SelectionTest(LoopTestCase):
    def classify(self):
        self.write_fakes()
        loop = self.mod.Loop(self.mod.parse_args(["--repo", self.repo, "--worktree", self.worktree, "--log-dir", self.logs]))
        return loop.classify(loop.list_tasks())

    def test_order_and_exclusions(self) -> None:
        self.task(10, milestone="M2")
        self.task(11, milestone="M1")
        self.task(12, milestone="M1", extra_labels=["行き詰まり"])
        self.task(13, milestone="M1", extra_labels=["ループ対象外"])
        self.task(14, milestone="M1", extra_labels=["保留"])
        self.add_issue(15, ["実装タスク"], milestone="M1")  # 難易度が無い
        self.task(16, milestone="M1", body="- 前提: #11, #30\n")  # #11 は開いている
        self.task(17, milestone="M1", body="- 前提: #30\n")  # #30 は閉じている
        self.task(18, milestone="M1", body="前提の行が無い\n")
        self.task(19, body="- 前提: なし\n")  # マイルストーンが無い
        self.task(20, milestone="M9", extra_labels=["AI起票"])
        self.task(21, milestone="M1", body="- 前提: #999\n")  # 存在しない Issue
        self.add_issue(30, ["実装タスク", "難易度:小"], state="CLOSED")
        self.add_issue(40, ["AI起票", "未判断"])  # タスクではない
        ready, skipped = self.classify()
        self.assertEqual([t.number for t in ready], [20, 11, 17, 18, 10, 19])
        reasons = {t.number: r for t, r in skipped}
        self.assertEqual(set(reasons), {12, 13, 14, 15, 16, 21})
        self.assertIn("行き詰まり", reasons[12])
        self.assertIn("難易度", reasons[15])
        self.assertIn("#11", reasons[16])
        self.assertNotIn("#30", reasons[16])
        self.assertIn("#999", reasons[21])

    def test_harder_label_wins(self) -> None:
        self.add_issue(1, ["実装タスク", "難易度:小", "難易度:大"])
        ready, _ = self.classify()
        self.assertEqual(ready[0].difficulty, "大")

    def test_dry_run_changes_nothing(self) -> None:
        self.task(1, difficulty="中")
        self.add_issue(5, ["AI起票", "未判断"])
        code = self.run_loop("--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("#1 [難易度:中]", self.output)
        self.assertIn("実装 sonnet・レビュー sonnet・引き上げ先 opus", self.output)
        self.assertIn("/implement-task 1 --loop", self.output)
        self.assertIn("/triage-issues --loop 5", self.output)
        self.assertFalse(os.path.exists(self.worktree))
        self.assertFalse(os.path.exists(self.logs))
        self.assertEqual(self.claude_calls(), [])
        for call in self.read_calls("gh-calls.jsonl"):
            self.assertIn(" ".join(call["argv"][:2]), ("issue list", "issue view"))

    def test_status(self) -> None:
        self.task(1)
        self.task(2, extra_labels=["行き詰まり"])
        self.add_issue(3, ["実装タスク", "難易度:小", "人の確認待ち"], state="CLOSED")
        self.add_issue(5, ["AI起票", "未判断"])
        code = self.run_loop("--status")
        self.assertEqual(code, 0)
        self.assertIn("残りのタスク: 2 件（いま選べるもの 1 件）", self.output)
        self.assertIn("#2 タスク2", self.output)
        self.assertIn("#3 タスク3", self.output)
        self.assertIn("#5 タスク5", self.output)
        self.assertEqual(self.claude_calls(), [])


class FlowTest(LoopTestCase):
    def test_pass_and_merge(self) -> None:
        self.task(1, body="- 前提: なし\n\n## 人が確かめること\n\n- 画面を見る\n")
        self.task(2, body="- 前提: #1\n\n## 人が確かめること\n\n- なし\n")
        self.script["implement"] = [
            {"structured_output": done(pr_title="A を足す", summary="A を足した。", fixed_problems=["誤記を直した"]), "files": {"a.txt": "A\n"}},
            {"structured_output": done(pr_title="B を足す"), "files": {"b.txt": "B\n"}},
        ]
        self.script["review"] = [review_pass("名前を短くできる"), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("選べるタスクが無い", self.output)
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.main_file("b.txt"), "B\n")
        self.assertEqual(self.issue(1)["state"], "CLOSED")
        self.assertEqual(self.issue(2)["state"], "CLOSED")
        self.assertIn("人の確認待ち", self.issue(1)["labels"])
        self.assertNotIn("人の確認待ち", self.issue(2)["labels"])
        self.assertEqual(self.issue(1)["comments"], [])

        pr = self.state["prs"]["100"]
        self.assertEqual(pr["state"], "MERGED")
        self.assertEqual(pr["title"], "A を足す")
        self.assertEqual(pr["head"], "task/1")
        self.assertTrue(pr["body"].startswith("Closes #1\n"))
        for text in ("A を足した。", "誤記を直した", "名前を短くできる", "1 回目（haiku）: 機械のチェック、レビュー、CI を通った"):
            self.assertIn(text, pr["body"])
        self.assertEqual(self.origin_branches(), ["main"])  # タスクのブランチは消してある

        implement = self.claude_calls("implement")
        self.assertEqual([c["prompt"] for c in implement], ["/implement-task 1 --loop", "/implement-task 2 --loop"])
        self.assertEqual([c["model"] for c in implement], ["haiku", "haiku"])
        review = self.claude_calls("review")
        self.assertEqual([c["prompt"] for c in review], ["/review-task 1 --loop", "/review-task 2 --loop"])
        self.assertEqual([c["model"] for c in review], ["sonnet", "sonnet"])
        for call in self.claude_calls():
            self.assertEqual(os.path.realpath(call["cwd"]), os.path.realpath(self.worktree))
            self.assertEqual(call["permission_mode"], "bypassPermissions")
            self.assertEqual(call["output_format"], "json")
            self.assertTrue(call["has_schema"])
            # 権限の設定は、作業ツリーの外に置いた main のコピーを渡す
            self.assertFalse(os.path.realpath(call["settings"]).startswith(os.path.realpath(self.worktree) + os.sep))
        self.assertIn('"Edit"', implement[0]["settings_text"])
        self.assertIn('"Read"', review[0]["settings_text"])
        # ユーザーのチェックアウトは触らない
        self.assertEqual(self.sh(["git", "status", "--porcelain"]), "")
        self.assertEqual(self.sh(["git", "rev-parse", "--abbrev-ref", "HEAD"]).strip(), "main")
        self.assertTrue(os.path.isfile(os.path.join(self.logs, os.listdir(self.logs)[0], "loop.log")))

    def test_max_tasks(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop("--max-tasks", "1")
        self.assertEqual(code, 0, self.output)
        self.assertIn("--max-tasks の 1 件に達した", self.output)
        self.assertEqual(self.issue(2)["state"], "OPEN")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_review_finding_then_fix(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(fixed_problems=["一つ目"], human_checks=["音を聞く"]), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(pr_title="直した題", fixed_problems=["一つ目", "二つ目"], human_checks=["音を聞く", "色を見る"]), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_fail("境界の値を扱っていない"), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.main_file("a.txt"), "A2\n")
        implement = self.claude_calls("implement")
        self.assertEqual(len(implement), 2)
        self.assertIsNone(implement[0]["resume"])
        self.assertEqual(implement[1]["resume"], "sess-1")
        self.assertTrue(implement[1]["prompt"].startswith("/implement-task 1 --loop fix\n\n"))
        self.assertIn("境界の値を扱っていない", implement[1]["prompt"])
        pr = self.state["prs"]["100"]
        self.assertEqual(pr["title"], "直した題")
        self.assertEqual(pr["body"].count("一つ目"), 1)
        self.assertIn("二つ目", pr["body"])
        self.assertIn("1 回目（haiku）: レビューで直す指摘が 1 件あった", pr["body"])
        # 実装中に足した「人が確かめること」は、重複なしでコメントに書き、ラベルを付ける
        self.assertIn("人の確認待ち", self.issue(1)["labels"])
        comments = self.issue(1)["comments"]
        self.assertEqual(len(comments), 1)
        self.assertRegex(comments[0], r"^人が確かめることを足した（\d{4}-\d{2}-\d{2}、Claude の判断）。\n\n- 音を聞く\n- 色を見る$")

    def test_check_failure_then_fix(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n", "FAIL_DOCS": "x\n"}},
            {"structured_output": done(), "delete": ["FAIL_DOCS"]},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertEqual(len(implement), 2)
        self.assertIn("機械のチェックの失敗: documents の検査", implement[1]["prompt"])
        self.assertIn("FAIL_DOCS がある", implement[1]["prompt"])
        self.assertEqual(len(self.claude_calls("review")), 1)  # チェックが落ちた回はレビューしない
        self.assertIsNone(self.main_file("FAIL_DOCS"))
        self.assertEqual(self.main_file("a.txt"), "A\n")

    def test_stuck_after_escalation_and_continue(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": f"A{i}\n"}} for i in range(4)]
        self.script["implement"].append({"structured_output": done(), "files": {"b.txt": "B\n"}})
        self.script["review"] = [review_fail("まだ違う")] * 4 + [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertEqual([c["model"] for c in implement], ["haiku", "haiku", "haiku", "sonnet", "haiku"])
        self.assertEqual([c["resume"] for c in implement], [None, "sess-1", "sess-1", None, None])
        issue = self.issue(1)
        self.assertEqual(issue["state"], "OPEN")
        self.assertIn("行き詰まり", issue["labels"])
        self.assertEqual(len(issue["comments"]), 1)
        self.assertRegex(issue["comments"][0], r"^行き詰まりにした（\d{4}-\d{2}-\d{2}、Claude の判断）。")
        for text in ("4 回目（sonnet）: レビューで直す指摘が 1 件あった", "`task/1`（push してある）", "まだ違う"):
            self.assertIn(text, issue["comments"][0])
        self.assertIsNone(self.main_file("a.txt"))
        self.assertIn("task/1", self.origin_branches())  # 途中の作業はブランチに残す
        self.assertEqual(self.state["prs"].get("100", {}).get("head"), "task/2")
        # 次のタスクへ進む
        self.assertEqual(self.issue(2)["state"], "CLOSED")
        self.assertEqual(self.main_file("b.txt"), "B\n")

    def test_large_task_has_no_escalation(self) -> None:
        self.task(1, difficulty="大")
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": f"A{i}\n"}} for i in range(3)]
        self.script["review"] = [review_fail("違う")] * 3
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual([c["model"] for c in self.claude_calls("implement")], ["opus", "opus", "opus"])
        self.assertEqual([c["model"] for c in self.claude_calls("review")], ["opus", "opus", "opus"])
        self.assertIn("行き詰まり", self.issue(1)["labels"])

    def test_stop_after_three_stuck_in_a_row(self) -> None:
        for number in (1, 2, 3, 4):
            self.task(number, difficulty="大")
        self.script["implement_default"] = {"structured_output": done(), "files": {"a.txt": "A\n"}}
        self.script["review_default"] = review_fail("違う")
        code = self.run_loop()
        self.assertEqual(code, 1, self.output)
        self.assertIn("行き詰まりが 3 件続いた", self.output)
        self.assertEqual([n for n in (1, 2, 3, 4) if "行き詰まり" in self.issue(n)["labels"]], [1, 2, 3])

    def test_claude_failure_stops_without_label(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [
            {"exit": 1, "is_error": True, "result": "usage limit reached", "api_error_status": 429, "files": {"a.txt": "A\n"}},
        ]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("仕組みの失敗", self.output)
        self.assertIn("429", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(self.issue(1)["state"], "OPEN")
        self.assertEqual(len(self.claude_calls()), 1)  # 次のタスクへ進まない
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.origin_branches(), ["main"])
        # 作業ツリーの変更は捨ててある
        self.assertEqual(self.sh(["git", "status", "--porcelain"], cwd=self.worktree), "")
        self.assertFalse(os.path.exists(os.path.join(self.worktree, "a.txt")))

    def test_is_error_with_exit_zero_stops(self) -> None:
        self.task(1)
        self.script["implement"] = [{"is_error": True, "result": "error_max_turns"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])

    def test_unreadable_output_stops(self) -> None:
        self.task(1)
        self.script["implement"] = [{"raw_stdout": "これは JSON ではない"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("JSON として読めません", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])

    def test_review_failure_stops_without_label(self) -> None:
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [{"exit": 1, "is_error": True, "result": "overloaded"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.state["prs"], {})

    def test_gh_failure_stops_without_label(self) -> None:
        self.task(1)
        self.state["fail"] = {"pr create": "HTTP 502"}
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("HTTP 502", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertIsNone(self.main_file("a.txt"))

    def test_protected_paths_are_reverted_without_fix_round(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {
                "structured_output": done(),
                "files": {
                    "a.txt": "A\n",
                    "CLAUDE.md": "書き換えた\n",
                    ".claude/loop-settings.json": '{"permissions": {"allow": ["Bash"]}}\n',
                    ".claude/skills/new/SKILL.md": "新しいスキル\n",
                    "tools/implement-loop.py": "print('x')\n",
                },
            },
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.claude_calls("implement")), 1)  # 直しに回さない
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.main_file("CLAUDE.md"), "決まり\n")
        self.assertIn('"Edit"', self.main_file(".claude/loop-settings.json"))
        self.assertIsNone(self.main_file(".claude/skills/new/SKILL.md"))
        self.assertIsNone(self.main_file("tools/implement-loop.py"))
        body = self.state["prs"]["100"]["body"]
        self.assertIn("保護するパスを変えていたので、スクリプトが元に戻した", body)
        for path in ("`CLAUDE.md`", "`.claude/loop-settings.json`", "`.claude/skills/new/SKILL.md`", "`tools/implement-loop.py`"):
            self.assertIn(path, body)
        # レビューに渡す権限の設定は、書き換えられる前の main のもの
        self.assertIn('"Read"', self.claude_calls("review")[0]["settings_text"])

    def test_only_protected_changes_count_as_no_change(self) -> None:
        self.task(1, difficulty="大")
        self.script["implement_default"] = {"structured_output": done(), "files": {"CLAUDE.md": "書き換えた\n"}}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("変更が無かった", self.issue(1)["comments"][0])
        self.assertEqual(self.claude_calls("review"), [])
        self.assertEqual(self.main_file("CLAUDE.md"), "決まり\n")

    def test_protected_path_left_before_push_is_not_merged(self) -> None:
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n", "CLAUDE.md": "書き換えた\n"}}]
        self.script["review"] = [review_pass()]
        self.mod.Loop.revert_protected = lambda loop, paths: None  # 戻す処理が効かなかったことにする
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("保護するパスの変更が残っている", self.issue(1)["comments"][0])
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.main_file("CLAUDE.md"), "決まり\n")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_blocked_adds_prerequisite(self) -> None:
        self.task(1, body="## 概要\n\nx\n\n- 前提: なし\n")
        self.add_issue(50, ["AI起票", "未判断", "仕様"])
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="戻る先が決まっていない。", blocking_issues=[50, 1]), "files": {"a.txt": "A\n"}},
        ]
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        issue = self.issue(1)
        self.assertEqual(issue["body"], "## 概要\n\nx\n\n- 前提: #50\n")
        self.assertEqual(issue["state"], "OPEN")
        self.assertNotIn("行き詰まり", issue["labels"])
        self.assertRegex(issue["comments"][0], r"^判断待ちにした（\d{4}-\d{2}-\d{2}、Claude の判断）。")
        self.assertIn("戻る先が決まっていない。", issue["comments"][0])
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertEqual(len(self.claude_calls("implement")), 1)  # 同じ実行では選び直さない
        self.assertIn("判断待ち: 1 件", self.output)

    def test_blocked_without_issue_or_line_is_stuck(self) -> None:
        self.task(1, difficulty="大")
        self.task(2, difficulty="大", body="前提の行が無い\n")
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="分からない。")},
            {"structured_output": done(status="blocked", blocked_reason="決まっていない。", blocking_issues=[50])},
        ]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        for number, text in ((1, "分からない。"), (2, "決まっていない。")):
            self.assertIn("行き詰まり", self.issue(number)["labels"])
            self.assertIn(text, self.issue(number)["comments"][0])
            self.assertIn("変更が無いので push していない", self.issue(number)["comments"][0])
        self.assertEqual(self.issue(2)["body"], "前提の行が無い\n")

    def test_blocked_by_permission_denial_stops_without_label(self) -> None:
        self.task(1)
        self.task(2)
        denial = [{"tool_name": "Bash", "tool_input": {"command": "gh issue view 1; git branch --show-current"}}]
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="Bash が許可されない。"), "permission_denials": denial, "files": {"a.txt": "A\n"}},
        ]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("仕組みの失敗", self.output)
        self.assertIn("権限で拒否された操作があり", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(len(self.claude_calls()), 1)  # 次のタスクへ進まない
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertFalse(os.path.exists(os.path.join(self.worktree, "a.txt")))

    def test_commit_by_claude_stops_without_label(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}, "commit": True},
        ]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("仕組みの失敗", self.output)
        self.assertIn("HEAD が変わりました", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(len(self.claude_calls()), 1)  # レビューにも次のタスクにも進まない
        self.assertEqual(self.origin_branches(), ["main"])

    def test_blocked_with_issue_and_denial_waits_for_decision(self) -> None:
        self.task(1, body="## 概要\n\nx\n\n- 前提: なし\n")
        self.add_issue(50, ["AI起票", "未判断", "仕様"])
        denial = [{"tool_name": "Bash", "tool_input": {"command": "git branch --show-current"}}]
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="決まっていない。", blocking_issues=[50]), "permission_denials": denial},
        ]
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.issue(1)["body"], "## 概要\n\nx\n\n- 前提: #50\n")
        self.assertNotIn("行き詰まり", self.issue(1)["labels"])

    def test_no_ci_checks_means_no_merge(self) -> None:
        self.task(1)
        self.task(2)
        self.state["checks"] = []
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("CI のチェックが 1 つも報告されません", self.output)
        self.assertEqual(self.state["prs"]["100"]["state"], "OPEN")
        self.assertEqual(self.gh_calls("pr merge"), [])
        self.assertIsNone(self.main_file("a.txt"))
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["state"], "OPEN")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_ci_appears_late_and_pending_then_pass(self) -> None:
        self.task(1)
        self.mod.CI_APPEAR_TIMEOUT = 5
        self.state["checks_queue"] = [[], PENDING, PENDING]
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.gh_calls("pr checks")), 4)
        self.assertEqual(self.main_file("a.txt"), "A\n")

    def test_ci_failure_then_fix_reuses_pull_request(self) -> None:
        self.task(1)
        self.state["checks_queue"] = [FAILING]
        self.state["run_log"] = "test_x が失敗した"
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertIn("## CI の失敗", implement[1]["prompt"])
        self.assertIn("test_x が失敗した", implement[1]["prompt"])
        self.assertEqual(len(self.gh_calls("pr create")), 1)
        self.assertEqual(list(self.state["prs"]), ["100"])
        self.assertEqual(self.state["prs"]["100"]["state"], "MERGED")
        self.assertEqual(self.main_file("a.txt"), "A2\n")
        self.assertIn("1 回目（haiku）: CI で失敗した", self.state["prs"]["100"]["body"])

    def test_ci_failure_until_stuck_leaves_pull_request_open(self) -> None:
        self.task(1, difficulty="大")
        self.state["checks"] = FAILING
        self.script["implement_default"] = {"structured_output": done(), "files": {"a.txt": "A\n"}}
        self.script["review_default"] = review_pass()
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("PR #100 が開いたままになっている", self.issue(1)["comments"][0])
        self.assertEqual(self.state["prs"]["100"]["state"], "OPEN")
        self.assertEqual(self.gh_calls("pr merge"), [])
        self.assertIsNone(self.main_file("a.txt"))

    def test_conflict_restarts_once_from_new_main(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"shared.txt": "タスクの変更\n"}, "push_main": {"shared.txt": "ほかの変更\n"}},
            {"structured_output": done(), "files": {"shared.txt": "ほかの変更\nタスクの変更\n"}},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertEqual([c["prompt"] for c in implement], ["/implement-task 1 --loop", "/implement-task 1 --loop"])
        self.assertEqual([c["resume"] for c in implement], [None, None])
        self.assertEqual(self.main_file("shared.txt"), "ほかの変更\nタスクの変更\n")
        self.assertIn("最初からやり直した", self.state["prs"]["100"]["body"])
        self.assertNotIn("行き詰まり", self.issue(1)["labels"])

    def test_second_conflict_is_stuck(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"shared.txt": "タスクの変更\n"}, "push_main": {"shared.txt": "ほかの変更 1\n"}},
            {"structured_output": done(), "files": {"shared.txt": "タスクの変更\n"}, "push_main": {"shared.txt": "ほかの変更 2\n"}},
        ]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.claude_calls("implement")), 2)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("また衝突した", self.issue(1)["comments"][0])
        self.assertEqual(self.main_file("shared.txt"), "ほかの変更 2\n")

    def test_main_moving_without_conflict_is_merged_in(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}, "push_main": {"other.txt": "ほかの変更\n"}},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.main_file("other.txt"), "ほかの変更\n")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_triage_runs_for_untriaged_ai_issues(self) -> None:
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.add_issue(6, ["AI起票", "仕様", "保留"])
        self.add_issue(7, ["AI起票", "不具合", "実装タスク", "難易度:小", "未判断"])
        self.task(8)
        self.script["triage_default"] = {"result": "仕分けた", "files": {"stray.txt": "仕分けが書いた\n"}}
        self.script["implement"] = [
            {"structured_output": done(summary="不具合を直した。", fixed_problems=["近くの誤記"]), "files": {"a.txt": "A\n"}},
            {"structured_output": done(), "files": {"b.txt": "B\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        triage = self.claude_calls("triage")
        self.assertEqual(len(triage), 1)  # 同じ Issue を、同じ実行で 2 回仕分けない
        self.assertEqual(triage[0]["prompt"], "/triage-issues --loop 5")
        self.assertEqual(triage[0]["model"], "sonnet")
        self.assertIn("gh issue", triage[0]["settings_text"])
        self.assertFalse(triage[0]["has_schema"])
        self.assertIn("仕分けの後に作業ツリーが変わっていた", self.output)
        self.assertIsNone(self.main_file("stray.txt"))
        # AI起票 のタスクは、ほかのタスクより先に選び、閉じるときにコメントを書いて `未判断` を外す
        self.assertEqual([c["prompt"] for c in self.claude_calls("implement")], ["/implement-task 7 --loop", "/implement-task 8 --loop"])
        issue = self.issue(7)
        self.assertEqual(issue["state"], "CLOSED")
        self.assertNotIn("未判断", issue["labels"])
        self.assertRegex(issue["comments"][0], r"^直したので閉じる（\d{4}-\d{2}-\d{2}、Claude の判断）。")
        for text in ("PR #100", "不具合を直した。", "近くの誤記"):
            self.assertIn(text, issue["comments"][0])
        self.assertEqual(self.issue(8)["comments"], [])

    def test_triage_skips_issues_that_open_tasks_wait_for(self) -> None:
        # #5 は、タスク #8 が「前提」で待っている、ユーザーの判断を求める Issue。ループでは仕分けない。
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.add_issue(6, ["AI起票", "未判断", "仕様"])
        self.add_issue(8, ["実装タスク", "難易度:中"], body="- 前提: #5\n")
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        triage = self.claude_calls("triage")
        self.assertEqual([c["prompt"] for c in triage], ["/triage-issues --loop 6"])
        self.assertEqual(self.claude_calls("implement"), [])
        self.assertEqual(self.issue(5)["state"], "OPEN")

    def test_triage_failure_stops(self) -> None:
        self.add_issue(5, ["AI起票", "未判断"])
        self.task(8)
        self.script["triage"] = [{"exit": 1, "is_error": True, "result": "limit"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertEqual(self.claude_calls("implement"), [])

    def test_missing_required_files_stops_before_any_call(self) -> None:
        self.task(1)
        self.sh(["git", "rm", "--quiet", ".claude/loop-triage-settings.json"])
        self.sh(["git", "commit", "--quiet", "-m", "消す"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.assertEqual(self.run_loop(), 2)
        self.assertIn(".claude/loop-triage-settings.json", self.output)
        self.assertEqual(self.claude_calls(), [])

    def test_missing_ci_definition_stops_before_any_call(self) -> None:
        self.task(1)
        self.sh(["git", "rm", "--quiet", ".github/workflows/ci.yml"])
        self.sh(["git", "commit", "--quiet", "-m", "消す"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("CI の定義がありません", self.output)
        self.assertEqual(self.claude_calls(), [])

    def test_worktree_is_reused_and_cleaned(self) -> None:
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 0, self.output)
        with open(os.path.join(self.worktree, "leftover.txt"), "w", encoding="utf-8") as handle:
            handle.write("前の実行の残り\n")
        with open(os.path.join(self.worktree, ".claude", "settings.local.json"), "w", encoding="utf-8") as handle:
            handle.write("{}\n")
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"b.txt": "B\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 0, self.output)
        self.assertEqual(self.main_file("b.txt"), "B\n")
        self.assertIsNone(self.main_file("leftover.txt"))
        self.assertFalse(os.path.exists(os.path.join(self.worktree, ".claude", "settings.local.json")))


if __name__ == "__main__":
    unittest.main()
