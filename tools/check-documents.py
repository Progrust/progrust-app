#!/usr/bin/env python3
"""documents/ 配下の OKF v0.3 バンドルを検査する。

使い方:
    python3 tools/check-documents.py [バンドルのルート]

引数を省略すると、このスクリプトから見たリポジトリの documents/ を検査する。
指摘は「パス:行: 規則名: 説明」の形で1行ずつ標準出力に出し、最後に件数を出す。

終了コード:
    0  指摘なし
    1  指摘あり
    2  バンドルのルートが無い

標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import json
import os
import re
import sys
import urllib.parse
from typing import Dict, List, Optional, Set, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

CONCEPT_KEYS = ("type", "title", "description", "status", "superseded_by")
REQUIRED_CONCEPT_KEYS = ("type", "title", "description")
ROOT_INDEX_KEYS = ("okf_version", "title", "description")
TYPES = (
    "Specification",
    "Acceptance Criteria",
    "Open Questions",
    "Glossary",
    "Decision",
    "Convention",
)
STATUSES = ("draft", "stable", "deprecated")
FORBIDDEN_NAMES = ("readme.md", "log.md")
ALLOWED_SCHEMES = ("http", "https", "mailto")

NAME_RE = re.compile(r"^[a-z0-9-]+$")
FM_KEY_RE = re.compile(r"^([A-Za-z0-9_-]+):(.*)$")
BLOCK_SCALAR_RE = re.compile(r"^[|>][-+0-9]*$")
INDEX_HEADING_RE = re.compile(r"^# \S")
INDEX_ENTRY_RE = re.compile(r"^\* \[(?P<name>.+?)\]\((?P<target>[^\s()]+)\) - (?P<desc>\S.*)$")
FENCE_OPEN_RE = re.compile(r"^\s*(`{3,}|~{3,})(.*)$")
H1_RE = re.compile(r"^ {0,3}#(?:[ \t]|$)")
SETEXT_H1_RE = re.compile(r"^ {0,3}=+[ \t]*$")
FOOTNOTE_RE = re.compile(r"\[\^[^\]\s]+\]")
SCHEME_RE = re.compile(r"^([A-Za-z][A-Za-z0-9+.-]*):")
LINK_RE = re.compile(
    r"!?\[(?:[^\[\]]|\[[^\[\]]*\])*\]"  # [文字]（1段までの入れ子の [] を許す）
    r"\(\s*"
    r"(<[^<>\n]*>|[^\s()]*(?:\([^\s()]*\)[^\s()]*)*)"  # リンク先
    r"(?:\s+(?:\"[^\"]*\"|'[^']*'|\([^()]*\)))?"  # タイトル
    r"\s*\)"
)
BUNDLE_PATH_RE = re.compile(
    r"(?<![A-Za-z0-9/._-])"
    r"((?:spec|decisions|conventions)/[^\s`'\"()\[\]]*?\.md)"
    r"(?!\.?[A-Za-z0-9_/-])"
)
BUNDLE_PATH_SKIP = ("*", "{", "<", "…")
ASCII_PUNCT = set("!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~")


class Finding:
    def __init__(self, path: str, line: Optional[int], rule: str, message: str) -> None:
        self.path = path
        self.line = line
        self.rule = rule
        self.message = message

    def sort_key(self) -> Tuple[str, int, str, str]:
        return (self.path, self.line or 0, self.rule, self.message)

    def format(self) -> str:
        location = f"{self.path}:{self.line}" if self.line is not None else self.path
        return f"{location}: {self.rule}: {self.message}"


class Doc:
    """1つの .md ファイルを読んだ結果。"""

    def __init__(self, rel: str, abspath: str) -> None:
        self.rel = rel  # バンドルのルートからの相対パス（/ 区切り）
        self.abspath = abspath
        self.name = rel.rsplit("/", 1)[-1]
        self.dir = rel.rsplit("/", 1)[0] if "/" in rel else ""
        self.lines: List[str] = []
        self.readable = True
        self.has_frontmatter = False
        self.frontmatter_closed = False
        self.body_start = 0  # 本文の最初の行の添字（0 始まり）
        # キー -> (1 始まりの行番号, 生の値)。重複したキーは最初のものを残す。
        self.fields: Dict[str, Tuple[int, str]] = {}
        self.key_order: List[Tuple[str, int]] = []
        # 書き方の誤りで値を読めなかったキー -> 行番号（必須キーの欠落として二重に指摘しないため）
        self.broken_keys: Dict[str, int] = {}
        self.title: Optional[str] = None  # 読めたときだけ入る
        self.description: Optional[str] = None

    @property
    def is_index(self) -> bool:
        return self.name == "index.md"

    @property
    def is_forbidden(self) -> bool:
        return self.name.lower() in FORBIDDEN_NAMES

    @property
    def is_concept(self) -> bool:
        return (
            self.readable
            and not self.is_index
            and not self.is_forbidden
            and self.has_frontmatter
            and "type" in self.fields
        )


def strip_inline_code(line: str) -> str:
    """インラインコードの範囲と、ASCII 記号のバックスラッシュエスケープを空白に置き換える。"""
    return split_inline_code(line)[0]


def split_inline_code(line: str) -> Tuple[str, List[str]]:
    """(インラインコードとエスケープを空白に置き換えた行, インラインコードの中身の一覧) を返す。"""
    out: List[str] = []
    spans: List[str] = []
    i = 0
    n = len(line)
    while i < n:
        c = line[i]
        if c == "\\" and i + 1 < n and line[i + 1] in ASCII_PUNCT:
            out.append("  ")
            i += 2
            continue
        if c == "`":
            j = i
            while j < n and line[j] == "`":
                j += 1
            run = j - i
            k = j
            end = -1
            while k < n:
                if line[k] == "`":
                    m = k
                    while m < n and line[m] == "`":
                        m += 1
                    if m - k == run:
                        end = m
                        break
                    k = m
                else:
                    k += 1
            if end >= 0:
                spans.append(line[j:end - run])
                out.append(" ")
                i = end
            else:
                out.append(line[i:j])
                i = j
            continue
        out.append(c)
        i += 1
    return "".join(out), spans


def prose_lines(lines: List[str], start: int):
    """本文の行をコードフェンスの外と中に分ける。

    (フェンスの外の行, フェンスの中の行, 閉じていないフェンスの開始添字か -1) を返す。
    行は (添字, 生の行)。フェンスの中の行に、開始と終了の行は含めない。
    """
    result: List[Tuple[int, str]] = []
    fenced: List[Tuple[int, str]] = []
    fence_char = ""
    fence_len = 0
    fence_start = -1
    for idx in range(start, len(lines)):
        line = lines[idx]
        if fence_char:
            stripped = line.strip()
            if (
                len(stripped) >= fence_len
                and stripped == fence_char * len(stripped)
            ):
                fence_char = ""
            else:
                fenced.append((idx, line))
            continue
        match = FENCE_OPEN_RE.match(line)
        if match:
            marker, info = match.group(1), match.group(2)
            if not (marker[0] == "`" and "`" in info):
                fence_char = marker[0]
                fence_len = len(marker)
                fence_start = idx
                continue
        result.append((idx, line))
    return result, fenced, (fence_start if fence_char else -1)


class Checker:
    def __init__(self, bundle: str) -> None:
        self.bundle = os.path.abspath(bundle)
        self.findings: List[Finding] = []
        self.docs: Dict[str, Doc] = {}
        self.paths: Set[str] = {""}  # バンドル内の全ファイルとディレクトリ（"" はルート）
        self.dirs: List[str] = [""]
        self.dir_entries: Dict[str, List[str]] = {}
        self.md_dirs: Set[str] = set()  # 配下のどこかに .md を持つディレクトリ

    # ---- 出力 -------------------------------------------------------------

    def display(self, rel: str, is_dir: bool = False) -> str:
        full = os.path.join(self.bundle, rel) if rel else self.bundle
        shown = None
        for base in (REPO_ROOT, os.getcwd()):
            relpath = os.path.relpath(full, base)
            if relpath != ".." and not relpath.startswith(".." + os.sep):
                shown = relpath
                break
        if shown is None:
            shown = full
        shown = shown.replace(os.sep, "/")
        if is_dir and not shown.endswith("/"):
            shown += "/"
        return shown

    def add(self, rel: str, line: Optional[int], rule: str, message: str, is_dir: bool = False) -> None:
        self.findings.append(Finding(self.display(rel, is_dir), line, rule, message))

    # ---- 走査 -------------------------------------------------------------

    def scan(self) -> None:
        for dirpath, dirnames, filenames in os.walk(self.bundle):
            dirnames[:] = sorted(
                d for d in dirnames
                if not d.startswith(".") and not os.path.islink(os.path.join(dirpath, d))
            )
            rel_dir = os.path.relpath(dirpath, self.bundle).replace(os.sep, "/")
            if rel_dir == ".":
                rel_dir = ""
            names = [f for f in filenames if not f.startswith(".")]
            self.dir_entries[rel_dir] = sorted(dirnames + names)
            for d in dirnames:
                child = f"{rel_dir}/{d}" if rel_dir else d
                self.paths.add(child)
                self.dirs.append(child)
            for f in sorted(names):
                rel = f"{rel_dir}/{f}" if rel_dir else f
                abspath = os.path.join(dirpath, f)
                if os.path.islink(abspath):
                    target = os.path.realpath(abspath)
                    if os.path.commonpath([target, os.path.realpath(self.bundle)]) != os.path.realpath(self.bundle):
                        continue  # バンドルの外を指すシンボリックリンクは読まない
                self.paths.add(rel)
                if f.lower().endswith(".md"):
                    self.docs[rel] = self.read_doc(rel, abspath)
                    parts = rel.split("/")[:-1]
                    self.md_dirs.add("")
                    for depth in range(1, len(parts) + 1):
                        self.md_dirs.add("/".join(parts[:depth]))

    def read_doc(self, rel: str, abspath: str) -> Doc:
        doc = Doc(rel, abspath)
        try:
            with open(abspath, "rb") as handle:
                raw = handle.read()
            text = raw.decode("utf-8")
        except (OSError, UnicodeDecodeError) as exc:
            doc.readable = False
            self.add(rel, 1, "encoding", f"UTF-8 として読めません（{exc}）。UTF-8 で保存し直してください。")
            return doc
        if text.startswith("﻿"):
            self.add(rel, 1, "encoding", "先頭に BOM があります。BOM なしの UTF-8 で保存し直してください。")
            text = text[1:]
        doc.lines = text.splitlines()
        if doc.lines and doc.lines[0] == "---":
            doc.has_frontmatter = True
            for idx in range(1, len(doc.lines)):
                if doc.lines[idx] == "---":
                    doc.frontmatter_closed = True
                    doc.body_start = idx + 1
                    break
        return doc

    # ---- frontmatter ------------------------------------------------------

    def parse_frontmatter(self, doc: Doc) -> None:
        """規則 4: frontmatter の各行が `キー: 値` の形で、キーが重複しないこと。"""
        rule = "frontmatter-syntax"
        if not doc.frontmatter_closed:
            self.add(doc.rel, 1, rule, "frontmatter を閉じる `---` の行がありません。キーの行の後に `---` だけの行を書いてください。")
            return
        for idx in range(1, doc.body_start - 1):
            number = idx + 1
            line = doc.lines[idx]
            if line.strip() == "":
                self.add(doc.rel, number, rule, "frontmatter に空行は書けません。この行を消してください。")
                continue
            if line.lstrip().startswith("#"):
                self.add(doc.rel, number, rule, "frontmatter にコメントは書けません。この行を消してください。")
                continue
            if line[0] in " \t":
                self.add(doc.rel, number, rule, "frontmatter に字下げした行（入れ子や複数行の値）は書けません。値は `キー: 値` の1行に収めてください。")
                continue
            match = FM_KEY_RE.match(line)
            if not match:
                self.add(doc.rel, number, rule, "frontmatter の行は `キー: 値` の形で書いてください（リストや入れ子は使えません）。")
                continue
            key, rest = match.group(1), match.group(2)
            if rest.strip() == "":
                self.add(doc.rel, number, rule, f"`{key}` の値がありません。入れ子や複数行の値は使えないので、`{key}: 値` の1行で書いてください。")
                continue
            if not rest.startswith(" ") or rest.startswith("  ") or rest.startswith(" \t"):
                self.add(doc.rel, number, rule, f"コロンの後には空白をちょうど1つ置いてください（`{key}: 値`）。")
                continue
            if rest != rest.rstrip():
                self.add(doc.rel, number, rule, "行末の空白を消してください。")
                continue
            if key in doc.fields or key in doc.broken_keys:
                first = doc.fields[key][0] if key in doc.fields else doc.broken_keys[key]
                self.add(doc.rel, number, rule, f"キー `{key}` が {first} 行目と重複しています。どちらか1つにしてください。")
                continue
            value = rest[1:]
            doc.key_order.append((key, number))
            if BLOCK_SCALAR_RE.match(value):
                self.add(doc.rel, number, rule, f"`{key}` に複数行の値（`{value}`）は使えません。値は `{key}: 値` の1行に収めてください。")
                doc.broken_keys[key] = number
                continue
            if value[0] in "[{":
                self.add(doc.rel, number, rule, f"`{key}` にリストや入れ子の値は使えません。値は1つの文字列にしてください。")
                doc.broken_keys[key] = number
                continue
            doc.fields[key] = (number, value)

    def check_key_order(self, doc: Doc, allowed: Tuple[str, ...], required: Tuple[str, ...], rule: str) -> None:
        order = ", ".join(allowed)
        last = -1
        for key, number in doc.key_order:
            if key not in allowed:
                self.add(doc.rel, number, rule, f"キー `{key}` は使えません。使えるキーは {order} だけです。")
                continue
            position = allowed.index(key)
            if position < last:
                self.add(doc.rel, number, rule, f"キー `{key}` の位置が違います。キーは {order} の順に並べてください。")
            last = max(last, position)
        for key in required:
            if key not in doc.fields and key not in doc.broken_keys:
                self.add(doc.rel, 1, rule, f"必須のキー `{key}` がありません。frontmatter に `{key}` を追加してください（順番は {order}）。")

    def check_json_string(self, doc: Doc, key: str) -> Optional[str]:
        """規則 7: 二重引用符で囲んだ、json.loads で読める空でない1行の文字列。"""
        if key not in doc.fields:
            return None
        number, raw = doc.fields[key]
        rule = "string-value"
        example = f'{key}: "…"'
        if len(raw) < 2 or not raw.startswith('"') or not raw.endswith('"'):
            self.add(doc.rel, number, rule, f"`{key}` の値は二重引用符で囲んでください（`{example}`）。")
            return None
        try:
            value = json.loads(raw)
        except ValueError as exc:
            self.add(doc.rel, number, rule, f"`{key}` の値が JSON の文字列として読めません（{exc}）。中の `\"` は `\\\"`、`\\` は `\\\\` と書いてください。")
            return None
        if not isinstance(value, str):
            self.add(doc.rel, number, rule, f"`{key}` の値は1つの文字列にしてください（`{example}`）。")
            return None
        if value.strip() == "":
            self.add(doc.rel, number, rule, f"`{key}` の値が空です。内容を書いてください。")
            return None
        if any(ord(ch) < 0x20 or ch in "  \x7f" for ch in value):
            self.add(doc.rel, number, rule, f"`{key}` の値に改行やタブなどの制御文字が入っています。1行の文字列にしてください。")
            return None
        return value

    def check_concept_frontmatter(self, doc: Doc) -> None:
        self.check_key_order(doc, CONCEPT_KEYS, REQUIRED_CONCEPT_KEYS, "frontmatter-keys")

        # 規則 6: type
        concept_type = None
        if "type" in doc.fields:
            number, raw = doc.fields["type"]
            choices = "、".join(TYPES)
            if raw in TYPES:
                concept_type = raw
            elif len(raw) >= 2 and raw[0] in "\"'" and raw[-1] == raw[0] and raw[1:-1] in TYPES:
                self.add(doc.rel, number, "type-value", f"`type` の値は引用符で囲まずに書いてください（`type: {raw[1:-1]}`）。")
                concept_type = raw[1:-1]
            else:
                self.add(doc.rel, number, "type-value", f"`type` の値 `{raw}` は使えません。{choices} のどれかを引用符なしで書いてください。")

        # 規則 7: title と description
        doc.title = self.check_json_string(doc, "title")
        doc.description = self.check_json_string(doc, "description")
        self.check_frontmatter_text(doc, "title", doc.title)
        self.check_frontmatter_text(doc, "description", doc.description)

        # 規則 8: status と superseded_by
        is_decision = concept_type == "Decision"
        status = None
        if "status" in doc.fields:
            number, raw = doc.fields["status"]
            if concept_type is not None and not is_decision:
                self.add(doc.rel, number, "status-value", f"`status` は `type: Decision` の concept でだけ使えます。この行を消してください。")
            elif raw in STATUSES:
                status = raw
            elif len(raw) >= 2 and raw[0] in "\"'" and raw[-1] == raw[0] and raw[1:-1] in STATUSES:
                self.add(doc.rel, number, "status-value", f"`status` の値は引用符で囲まずに書いてください（`status: {raw[1:-1]}`）。")
                status = raw[1:-1]
            else:
                self.add(doc.rel, number, "status-value", f"`status` の値 `{raw}` は使えません。draft、stable、deprecated のどれかを引用符なしで書いてください。")
        if "superseded_by" in doc.fields:
            number, raw = doc.fields["superseded_by"]
            rule = "superseded-by"
            if concept_type is not None and not is_decision:
                self.add(doc.rel, number, rule, "`superseded_by` は `type: Decision` の concept でだけ使えます。この行を消してください。")
            elif status != "deprecated":
                self.add(doc.rel, number, rule, "`superseded_by` は `status: deprecated` のときだけ書けます。`status: deprecated` を書くか、この行を消してください。")
            if raw[:1] in "\"'":
                self.add(doc.rel, number, rule, "`superseded_by` の値は引用符で囲まずに書いてください（例: `superseded_by: /decisions/0002-example.md`）。")
            elif not raw.startswith("/"):
                self.add(doc.rel, number, rule, f"`superseded_by` の値はバンドルのルートからのパスで、`/` から始めてください（例: `/{self.suggest_root_path(doc, raw)}`）。")
            else:
                target = os.path.normpath(urllib.parse.unquote(raw[1:])).replace(os.sep, "/")
                target_doc = self.docs.get(target)
                if target_doc is None or not target_doc.is_concept:
                    self.add(doc.rel, number, rule, f"`superseded_by` の `{raw}` に concept がありません。置き換え先の concept のパスを `/` から書いてください。")
                elif target == doc.rel:
                    self.add(doc.rel, number, rule, "`superseded_by` が自分自身を指しています。置き換え先の concept のパスを書いてください。")

    def suggest_root_path(self, doc: Doc, raw: str) -> str:
        resolved = self.resolve(doc.dir, raw)
        return resolved if resolved else "decisions/0002-example.md"

    def check_root_index_frontmatter(self, doc: Doc) -> None:
        """規則 9: ルートの index.md の frontmatter。"""
        rule = "root-index-frontmatter"
        if not doc.has_frontmatter:
            self.add(doc.rel, 1, rule, 'バンドルのルートの index.md には frontmatter が必要です。1行目から `---`、`okf_version: "0.3"`、`title: "…"`、`description: "…"`、`---` を書いてください。')
            return
        self.parse_frontmatter(doc)
        if not doc.frontmatter_closed:
            return
        self.check_key_order(doc, ROOT_INDEX_KEYS, ROOT_INDEX_KEYS, rule)
        if "okf_version" in doc.fields:
            number, raw = doc.fields["okf_version"]
            if raw != '"0.3"':
                self.add(doc.rel, number, rule, f'`okf_version` の値は二重引用符つきの `"0.3"` にしてください（`okf_version: "0.3"`）。')
        doc.title = self.check_json_string(doc, "title")
        doc.description = self.check_json_string(doc, "description")
        self.check_frontmatter_text(doc, "title", doc.title)
        self.check_frontmatter_text(doc, "description", doc.description)

    def check_frontmatter_text(self, doc: Doc, key: str, value: Optional[str]) -> None:
        """規則 16〜18 を title と description にも当てる（インラインコードの中は除く）。"""
        if value is None:
            return
        number = doc.fields[key][0]
        text, spans = split_inline_code(value)
        self.check_text_rules(doc, number, text, where=f"`{key}` の値")
        for span in spans:
            self.check_bundle_paths(doc, number, span, where=f"`{key}` の値のインラインコード")

    # ---- 本文 -------------------------------------------------------------

    def check_text_rules(self, doc: Doc, number: int, text: str, where: str = "") -> None:
        prefix = f"{where}に" if where else ""
        if "§" in text:
            self.add(doc.rel, number, "section-sign", f"{prefix}`§` があります。節番号で参照せず、参照先の文書への Markdown リンク（例: `[会話履歴](history.md)`）で書いてください。")
        for match in FOOTNOTE_RE.finditer(text):
            self.add(doc.rel, number, "footnote", f"{prefix}脚注の記法 `{match.group(0)}` があります。このプロジェクトでは脚注を使いません。本文に書くかリンクにしてください。")
        if "./work" in text:
            self.add(doc.rel, number, "work-reference", f"{prefix}`./work` があります。一時作業用のディレクトリは参照できません。参照を消すか、必要な内容を documents/ の中の文書に書いてください。")

    def check_body(self, doc: Doc) -> None:
        """規則 15〜19: 本文（frontmatter の後）を、コードフェンスとインラインコードの外で調べる。"""
        start = doc.body_start if doc.has_frontmatter else 0
        lines, fenced, unclosed = prose_lines(doc.lines, start)
        if unclosed >= 0:
            self.add(doc.rel, unclosed + 1, "fence-unclosed", "このコードフェンスが閉じていません。開始と同じ記号を同じ数以上並べた行で閉じてください（閉じないとその先の本文が検査されません）。")
        for idx, line in fenced:
            number = idx + 1
            if "§" in line:
                self.add(doc.rel, number, "section-sign", "コードフェンスの中に `§` があります。節番号で参照せず、参照先の文書をバンドルのルートからのパス（例: `spec/qa-chat/history.md`）で書いてください。")
            self.check_bundle_paths(doc, number, line, where="コードフェンスの中")
        previous_text = False
        for idx, line in lines:
            number = idx + 1
            text, spans = split_inline_code(line)
            for match in LINK_RE.finditer(text):
                self.check_link(doc, number, match.group(1))
            self.check_text_rules(doc, number, text)
            for span in spans:
                self.check_bundle_paths(doc, number, span, where="インラインコードの中")
            if doc.is_concept:
                if H1_RE.match(line):
                    self.add(doc.rel, number, "heading-level", "concept の本文に深さ1の見出し（`# `）は書けません。見出しは title が担うので、この行を消すか `## ` に変えてください。")
                elif previous_text and SETEXT_H1_RE.match(line):
                    self.add(doc.rel, number, "heading-level", "`===` の下線で深さ1の見出しになっています。concept の本文の見出しは `## ` から書いてください。")
            stripped = line.strip()
            previous_text = bool(stripped) and not stripped.startswith(("#", ">", "|"))

    def check_bundle_paths(self, doc: Doc, number: int, text: str, where: str) -> None:
        """規則 bundle-path: リンクが書けない場所のパス（spec/…/x.md など）が、バンドルのルートから見て実在すること。"""
        for match in BUNDLE_PATH_RE.finditer(text):
            path = match.group(1)
            if any(ch in path for ch in BUNDLE_PATH_SKIP):
                continue
            resolved = self.resolve("", path)
            if resolved is None:
                self.add(doc.rel, number, "bundle-path", f"{where}のパス `{path}` はバンドルの外を指しています。バンドルのルートからの、実在する文書のパスを書いてください。")
            elif resolved not in self.paths or resolved in self.dirs:
                hint = self.closest_hint(resolved, from_root=True)
                self.add(doc.rel, number, "bundle-path", f"{where}のパス `{path}` がバンドルにありません（バンドルのルートから探しました）。リンクの書けない場所では、他の文書をバンドルのルートからの実在するパスで書いてください。{hint}")

    def check_link(self, doc: Doc, number: int, target: str) -> None:
        """規則 15: リンク先がバンドルの中に実在すること。"""
        rule = "link-target"
        if target.startswith("<") and target.endswith(">"):
            target = target[1:-1]
        if target == "":
            self.add(doc.rel, number, rule, "リンク先が空です。リンク先のパスか URL を書いてください。")
            return
        if target.startswith("#"):
            return
        scheme = SCHEME_RE.match(target)
        if scheme:
            if scheme.group(1).lower() in ALLOWED_SCHEMES:
                return
            self.add(doc.rel, number, rule, f"リンク先 `{target}` のスキームは使えません。外部へのリンクは http:、https:、mailto: だけです。バンドル内の文書へは相対パスで書いてください。")
            return
        if target.startswith("//"):
            self.add(doc.rel, number, rule, f"リンク先 `{target}` はスキームを省いた URL です。`https://` から書いてください。")
            return
        resolved = self.resolve(doc.dir, target)
        if resolved is None:
            self.add(doc.rel, number, rule, f"リンク先 `{target}` はバンドル（documents/）の外を指しています。バンドルの中の文書へのリンクにするか、リンクを消してください。")
        elif resolved not in self.paths:
            hint = self.closest_hint(resolved)
            self.add(doc.rel, number, rule, f"リンク先 `{target}` が見つかりません（`{resolved or '/'}` を探しました）。{hint}")

    def resolve(self, base_dir: str, target: str) -> Optional[str]:
        """リンク先をバンドルのルートからの相対パスにする。外に出るなら None。"""
        clean = urllib.parse.unquote(target.split("#", 1)[0].split("?", 1)[0])
        if clean.startswith("/"):
            joined = clean.lstrip("/")
        else:
            joined = f"{base_dir}/{clean}" if base_dir else clean
        parts: List[str] = []
        for part in joined.split("/"):
            if part in ("", "."):
                continue
            if part == "..":
                if not parts:
                    return None
                parts.pop()
            else:
                parts.append(part)
        return "/".join(parts)

    def closest_hint(self, resolved: str, from_root: bool = False) -> str:
        lowered = resolved.lower()
        for path in self.paths:
            if path.lower() == lowered:
                return f"大文字小文字が違う `{path}` ならあります。パスは大文字小文字を区別します。"
        parts = resolved.split("/")
        if any(p.startswith(".") for p in parts):
            return "`.` で始まるファイルやディレクトリはバンドルに含まれません。"
        if from_root:
            return "パスの綴りと、移動や改名で古いパスのまま残っていないかを確かめてください。"
        return "パスの綴りと、そのファイルのディレクトリからの相対位置を確かめてください。"

    # ---- index.md ---------------------------------------------------------

    def check_index(self, doc: Doc) -> None:
        is_root = doc.dir == ""
        if is_root:
            self.check_root_index_frontmatter(doc)
            if doc.has_frontmatter and not doc.frontmatter_closed:
                return
            start = doc.body_start if doc.has_frontmatter else 0
        else:
            if doc.has_frontmatter:
                self.add(doc.rel, 1, "index-frontmatter", "ルート以外の index.md は frontmatter を持てません。先頭の `---` から閉じる `---` までを消してください。")
                start = doc.body_start if doc.frontmatter_closed else 1
            else:
                start = 0

        # 規則 14 のための、載るべきもの
        concepts = {
            d.name: d for d in self.docs.values()
            if d.dir == doc.dir and d.is_concept
        }
        subdirs = [
            p for p in self.md_dirs
            if p and (p.rsplit("/", 1)[0] if "/" in p else "") == doc.dir
        ]
        expected: Dict[str, str] = {name: name for name in concepts}
        for sub in subdirs:
            expected[sub.rsplit("/", 1)[-1] + "/index.md"] = sub
        seen: Dict[str, int] = {}

        first = True
        for idx in range(start, len(doc.lines)):
            number = idx + 1
            line = doc.lines[idx]
            text = strip_inline_code(line)
            self.check_text_rules(doc, number, text)
            if line.strip() == "":
                if line:
                    self.add(doc.rel, number, "index-line", "空白だけの行があります。空白を消して空行にしてください。")
                continue
            if INDEX_HEADING_RE.match(line):
                first = False
                for span in split_inline_code(line)[1]:
                    self.check_bundle_paths(doc, number, span, where="インラインコードの中")
                for link in LINK_RE.finditer(text):
                    self.check_link(doc, number, link.group(1))
                continue
            if first:
                self.add(doc.rel, number, "index-line", "index.md の本文は `# 見出し` の行から始めてください。")
                first = False
            match = INDEX_ENTRY_RE.match(line)
            if not match:
                self.add(doc.rel, number, "index-line", "index.md の本文に書けるのは `# 見出し` の行、空行、`* [名前](リンク先) - 説明` の行だけです。この行をその形に直すか、消してください。")
                continue
            self.check_index_entry(doc, number, match, concepts, expected, seen)

        dir_label = f"{doc.dir}/" if doc.dir else "バンドルのルート"
        for key in sorted(expected):
            if key in seen:
                continue
            if key in concepts:
                c = concepts[key]
                if c.title is not None and c.description is not None:
                    entry = f"`* [{c.title}]({key}) - {c.description}`"
                else:
                    entry = f"`* [title]({key}) - description`（{key} の frontmatter の値）"
                self.add(doc.rel, 1, "index-coverage", f"{dir_label}の concept `{key}` が載っていません。{entry} の行を追加してください。")
            else:
                self.add(doc.rel, 1, "index-coverage", f"{dir_label}のディレクトリ `{key.split('/')[0]}/` が載っていません。`* [名前]({key}) - 説明` の行を追加してください。")

    def check_index_entry(self, doc, number, match, concepts, expected, seen) -> None:
        name, target, desc = match.group("name"), match.group("target"), match.group("desc")
        # 規則 12: リンク先の形
        is_concept_form = "/" not in target and target.endswith(".md") and target != "index.md"
        is_subindex_form = (
            target.count("/") == 1
            and target.endswith("/index.md")
            and target.split("/")[0] not in ("", ".", "..")
        )
        if not (is_concept_form or is_subindex_form):
            hint = ""
            stripped = target[2:] if target.startswith("./") else target
            if stripped != target and ("/" not in stripped or stripped.endswith("/index.md")):
                hint = f"`./` を付けずに `{stripped}` と書いてください。"
            elif target.endswith("/") and target.count("/") == 1:
                hint = f"`{target}index.md` と書いてください。"
            else:
                hint = "同じディレクトリの concept（`history.md`）か、1段下のディレクトリの index（`video/index.md`）だけを載せられます。"
            self.add(doc.rel, number, "index-target", f"リンク先 `{target}` は index.md に載せられる形ではありません。{hint}")
            return
        # 規則 14: 2回目以降、実在しないもの
        if target in seen:
            self.add(doc.rel, number, "index-coverage", f"`{target}` は {seen[target]} 行目にも載っています。どちらか1行を消してください。")
            return
        seen[target] = number
        if target not in expected:
            rel_target = f"{doc.dir}/{target}" if doc.dir else target
            if is_concept_form and rel_target in self.docs:
                self.add(doc.rel, number, "index-target", f"`{target}` は concept ではない（frontmatter に type が無い）ので載せられません。その文書を concept にするか、この行を消してください。")
            elif is_subindex_form and target.split("/")[0] in self.dir_entries.get(doc.dir, []):
                self.add(doc.rel, number, "index-coverage", f"`{target}` がありません。ディレクトリ `{target.split('/')[0]}/` に .md が無いなら、この行を消してください。")
            else:
                self.add(doc.rel, number, "index-coverage", f"`{target}` は実在しません。リンク先を実在するファイルに直すか、この行を消してください。")
            return
        # 規則 13: 名前と説明が concept の title と description に一致する
        if is_concept_form:
            concept = concepts[target]
            if concept.title is not None and name != concept.title:
                self.add(doc.rel, number, "index-entry-text", f"名前が `{target}` の title と違います。`[{concept.title}]` にしてください。")
            if concept.description is not None and desc != concept.description:
                self.add(doc.rel, number, "index-entry-text", f"説明が `{target}` の description と違います。` - ` の後を「{concept.description}」にしてください。")

    # ---- ファイル名とディレクトリ ------------------------------------------------

    def check_names(self) -> None:
        """規則 2 と 3。"""
        for rel in self.dirs:
            if not rel:
                continue
            name = rel.rsplit("/", 1)[-1]
            if not NAME_RE.match(name):
                self.add(rel, None, "file-name", f"ディレクトリ名 `{name}` に英小文字・数字・ハイフン以外の文字があります。英小文字・数字・ハイフンだけの名前に変えてください。", is_dir=True)
        for rel, doc in self.docs.items():
            if doc.is_forbidden:
                continue
            stem, ext = doc.name[:-3], doc.name[-3:]
            if ext != ".md":
                self.add(rel, 1, "file-name", f"拡張子は小文字の `.md` にしてください（`{stem}.md`）。")
            if not NAME_RE.match(stem):
                self.add(rel, 1, "file-name", f"ファイル名 `{doc.name}` に英小文字・数字・ハイフン以外の文字があります。拡張子の前を英小文字・数字・ハイフンだけにしてください。")
        for rel_dir, entries in sorted(self.dir_entries.items()):
            for a, b in case_collisions(entries):
                path_a = f"{rel_dir}/{a}" if rel_dir else a
                path_b = f"{rel_dir}/{b}" if rel_dir else b
                is_dir = path_b in self.dirs
                self.add(path_b, None if is_dir else 1, "file-name", f"`{self.display(path_a, is_dir)}` と大文字小文字だけが違います。どちらかの名前を変えてください。", is_dir=is_dir)
        for rel in sorted(self.md_dirs | {""}):
            index = f"{rel}/index.md" if rel else "index.md"
            if index in self.docs:
                continue
            if rel:
                self.add(rel, None, "index-missing", "このディレクトリには .md があるのに index.md がありません。index.md を作り、concept と下位ディレクトリを `* [名前](リンク先) - 説明` の形で載せてください。", is_dir=True)
            else:
                self.add(rel, None, "index-missing", "バンドルのルートに index.md がありません。frontmatter（okf_version、title、description）を持つ index.md を作り、concept と下位ディレクトリを `* [名前](リンク先) - 説明` の形で載せてください。", is_dir=True)

    # ---- 全体 -------------------------------------------------------------

    def run(self) -> None:
        self.scan()
        # concept の判定（他のファイルの検査で使う）のため、先に frontmatter を読む
        for doc in self.docs.values():
            if doc.readable and doc.has_frontmatter and not doc.is_index and not doc.is_forbidden:
                self.parse_frontmatter(doc)
        self.check_names()
        # index.md の検査（規則 13）は concept の title と description を使うので、concept を先に調べる
        for rel in sorted(self.docs):
            doc = self.docs[rel]
            if not doc.readable or doc.is_index:
                continue
            if doc.is_forbidden:
                self.add(rel, 1, "file-kind", f"`{doc.name}` はバンドルに置けません。内容が必要なら concept（frontmatter に type を持つ .md）に移し、このファイルを消してください。")
                continue
            if not doc.has_frontmatter:
                self.add(rel, 1, "file-kind", "frontmatter が無いので concept ではありません。1行目から `---`、`type: …`、`title: \"…\"`、`description: \"…\"`、`---` を書いて concept にするか、バンドルの外へ移してください。")
                self.check_body(doc)
                continue
            if not doc.frontmatter_closed:
                continue
            self.check_concept_frontmatter(doc)
            self.check_body(doc)
        for rel in sorted(self.docs):
            doc = self.docs[rel]
            if doc.readable and doc.is_index:
                self.check_index(doc)


def case_collisions(names: List[str]) -> List[Tuple[str, str]]:
    """大文字小文字だけが違う名前の組を返す。"""
    pairs: List[Tuple[str, str]] = []
    first: Dict[str, str] = {}
    for name in sorted(names):
        key = name.lower()
        if key in first:
            pairs.append((first[key], name))
        else:
            first[key] = name
    return pairs


USAGE = "使い方: python3 tools/check-documents.py [--hook] [バンドルのルート]"


def main(argv: List[str]) -> int:
    hook = "--hook" in argv
    args = [a for a in argv if a != "--hook"]
    if len(args) > 1 or any(a.startswith("--") for a in args):
        print(USAGE, file=sys.stderr)
        return 2
    bundle = args[0] if args else os.path.join(REPO_ROOT, "documents")
    if not os.path.isdir(bundle):
        if hook:
            return 0  # documents/ が無い状態でセッションを止めない
        print(f"バンドルのルート `{bundle}` がありません（ディレクトリではありません）。", file=sys.stderr)
        return 2
    checker = Checker(bundle)
    checker.run()
    findings = sorted(checker.findings, key=Finding.sort_key)
    if hook and not findings:
        return 0
    out = sys.stderr if hook else sys.stdout
    for finding in findings:
        print(finding.format(), file=out)
    print(f"{len(checker.docs)} 個の .md ファイルを検査し、指摘は {len(findings)} 件でした。", file=out)
    if not findings:
        return 0
    return 2 if hook else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
