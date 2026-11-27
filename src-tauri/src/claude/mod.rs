//! `claude` の起動（spec/claude-integration/invocation.md）。
//!
//! アプリが `claude` を起動するときは、検出のための `claude --version` も `claude -p` の呼び出しも、
//! [`ClaudeEnv::command`] でプロセスを作る。渡す環境変数を、ここの 1 か所で決める。
//! `claude -p` を 1 回呼ぶ部品は [`call`] にある。

pub mod call;
mod stream;

pub use call::{Canceller, ClaudeCall, ClaudeOutcome, ClaudeRequest};
pub use stream::{ClaudeError, ErrorCategory};

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Command,
};

/// AI のモデルの値（`settings.json` の `models.chat`・`models.background`）として使えるか。
/// 空でなく、空白と制御文字を含まず、`-` で始まらない文字列だけを使える
/// （spec/screens/settings-screen.md「settings.json」、ADR 0258）。`update_settings` の検査と、
/// `claude -p` を起動する前の検査の両方が、これを使う。
pub fn is_valid_model(model: &str) -> bool {
    !model.is_empty()
        && !model.starts_with('-')
        && !model.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// `claude` に渡す環境変数の名前（spec/claude-integration/invocation.md「環境変数」）。
/// ほかの環境変数は渡さない。
pub const ENV_VAR_NAMES: [&str; 6] = ["HOME", "USER", "LOGNAME", "PATH", "SHELL", "TMPDIR"];

/// `claude` に渡す環境変数。[`ENV_VAR_NAMES`] のうち、アプリが受け取ったものだけを、
/// 受け取った値のまま持つ。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeEnv {
    vars: Vec<(OsString, OsString)>,
}

impl ClaudeEnv {
    /// アプリのプロセスが受け取った環境変数から作る。
    pub fn from_process() -> Self {
        Self::from_vars(std::env::vars_os())
    }

    /// 環境変数の名前と値の組から、[`ENV_VAR_NAMES`] のものだけを選んで作る。
    /// 同じ名前が 2 度あるときは、後のものを使う。
    pub fn from_vars<K, V>(vars: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        let mut selected: Vec<(OsString, OsString)> = Vec::new();
        for (name, value) in vars {
            let name = name.into();
            if !ENV_VAR_NAMES
                .iter()
                .any(|allowed| OsStr::new(allowed) == name)
            {
                continue;
            }
            selected.retain(|(existing, _)| *existing != name);
            selected.push((name, value.into()));
        }
        Self { vars: selected }
    }

    /// 渡す環境変数の名前と値。
    pub fn vars(&self) -> &[(OsString, OsString)] {
        &self.vars
    }

    /// `program`（`claude` の実行ファイル）を起動する [`Command`]。アプリが引き継いだ環境変数を
    /// 消し、この環境変数だけを渡す。引数や標準入出力は、呼び出し側が決める。
    pub fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command.env_clear();
        command.envs(self.vars.iter().map(|(name, value)| (name, value)));
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_values() {
        for model in [
            "opus",
            "sonnet",
            "haiku",
            "claude-opus-4-1",
            "claude-haiku-5-5[1m]",
            "a-",
        ] {
            assert!(is_valid_model(model), "{model:?}");
        }
        for model in [
            "",
            "-",
            "-p",
            "--bare",
            "two words",
            " opus",
            "opus\n",
            "op\tus",
            "opus\u{7f}",
            "opus\u{3000}",
        ] {
            assert!(!is_valid_model(model), "{model:?}");
        }
    }

    fn names(env: &ClaudeEnv) -> Vec<&str> {
        env.vars()
            .iter()
            .map(|(name, _)| name.to_str().unwrap())
            .collect()
    }

    #[test]
    fn only_the_six_variables_are_kept() {
        let env = ClaudeEnv::from_vars([
            ("HOME", "/Users/me"),
            ("ANTHROPIC_API_KEY", "secret"),
            ("USER", "me"),
            ("LOGNAME", "me"),
            ("CLAUDE_CONFIG_DIR", "/tmp/claude"),
            ("PATH", "/usr/bin:/bin"),
            ("LANG", "ja_JP.UTF-8"),
            ("SHELL", "/bin/zsh"),
            ("TMPDIR", "/var/folders/xx/T/"),
            ("HTTPS_PROXY", "http://proxy"),
        ]);
        assert_eq!(
            env.vars(),
            [
                ("HOME", "/Users/me"),
                ("USER", "me"),
                ("LOGNAME", "me"),
                ("PATH", "/usr/bin:/bin"),
                ("SHELL", "/bin/zsh"),
                ("TMPDIR", "/var/folders/xx/T/"),
            ]
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        );
    }

    /// アプリが受け取っていない変数は渡さない（空の値で足さない）。
    #[test]
    fn variables_the_app_did_not_receive_are_not_added() {
        let env = ClaudeEnv::from_vars([("HOME", "/Users/me"), ("PATH", "/usr/bin")]);
        assert_eq!(names(&env), ["HOME", "PATH"]);
        assert_eq!(ClaudeEnv::from_vars::<&str, &str>([]).vars(), []);
    }

    /// 名前は大文字と小文字を区別する。
    #[test]
    fn names_are_case_sensitive() {
        let env = ClaudeEnv::from_vars([("home", "/Users/me"), ("Path", "/usr/bin")]);
        assert_eq!(env.vars(), []);
    }

    #[test]
    fn a_repeated_name_uses_the_last_value() {
        let env = ClaudeEnv::from_vars([("HOME", "/a"), ("HOME", "/b")]);
        assert_eq!(env.vars(), [(OsString::from("HOME"), OsString::from("/b"))]);
    }

    #[test]
    fn from_process_keeps_only_the_six_variables() {
        let env = ClaudeEnv::from_process();
        let expected: Vec<(OsString, OsString)> = std::env::vars_os()
            .filter(|(name, _)| {
                ENV_VAR_NAMES
                    .iter()
                    .any(|allowed| OsStr::new(allowed) == name)
            })
            .collect();
        assert_eq!(env.vars(), expected);
    }

    #[test]
    fn the_command_passes_only_these_variables() {
        let env = ClaudeEnv::from_vars([("HOME", "/Users/me"), ("PATH", "/usr/bin")]);
        let command = env.command(Path::new("/usr/local/bin/claude"));
        assert_eq!(command.get_program(), "/usr/local/bin/claude");
        let envs: Vec<(&OsStr, Option<&OsStr>)> = command.get_envs().collect();
        assert_eq!(
            envs,
            [
                (OsStr::new("HOME"), Some(OsStr::new("/Users/me"))),
                (OsStr::new("PATH"), Some(OsStr::new("/usr/bin"))),
            ]
        );
    }
}
