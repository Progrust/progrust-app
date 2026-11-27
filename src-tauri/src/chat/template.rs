//! 指示文の `{{name}}` を値に置き換える（spec/claude-integration/prompts.md）。

use std::collections::BTreeSet;
use std::fmt;

/// 指示文の `{{name}}` を置き換えられなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    /// 指示文の `{{name}}` に、渡した値が無い（置き換えられずに残る）。
    Unfilled { name: String },
    /// 渡した値の名前が、指示文に無い。
    UnknownName { name: String },
    /// 指示文の `{{` が、`{{name}}` の形になっていない（閉じていない、名前が空、名前に使えない文字がある）。
    Malformed { offset: usize },
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unfilled { name } => write!(f, "指示文の {{{{{name}}}}} に入れる値が無い"),
            Self::UnknownName { name } => write!(f, "指示文に {{{{{name}}}}} が無い"),
            Self::Malformed { offset } => {
                write!(
                    f,
                    "指示文の {offset} バイト目の {{{{ が {{{{name}}}} の形でない"
                )
            }
        }
    }
}

impl std::error::Error for TemplateError {}

/// `template` の `{{name}}` を、`values` の同じ名前の値に置き換える。
///
/// 指示文を先頭から 1 回だけ読んで置き換えるので、値の中の `{{…}}` は置き換えの対象にならず、そのまま残る。
/// 指示文の `{{name}}` に値が無いとき、値の名前が指示文に無いとき、指示文の `{{` が `{{name}}` の形でないときは
/// エラーにする。名前は、英小文字・数字・`_` からなる。同じ名前が指示文に何度出てもよい。
pub fn fill(template: &str, values: &[(&str, &str)]) -> Result<String, TemplateError> {
    let mut output = String::with_capacity(template.len());
    let mut used = BTreeSet::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let offset = template.len() - rest.len() + start;
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or(TemplateError::Malformed { offset })?;
        let name = &after[..end];
        if !is_name(name) {
            return Err(TemplateError::Malformed { offset });
        }
        let (_, value) =
            values
                .iter()
                .find(|(key, _)| *key == name)
                .ok_or_else(|| TemplateError::Unfilled {
                    name: name.to_string(),
                })?;
        output.push_str(value);
        used.insert(name);
        rest = &after[end + 2..];
    }
    output.push_str(rest);

    if let Some((name, _)) = values.iter().find(|(name, _)| !used.contains(name)) {
        return Err(TemplateError::UnknownName {
            name: name.to_string(),
        });
    }
    Ok(output)
}

fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_every_placeholder() {
        let filled = fill(
            "# A\n{{a}}\n# B\n{{b_2}} と {{a}}",
            &[("a", "あ"), ("b_2", "い")],
        );
        assert_eq!(filled.unwrap(), "# A\nあ\n# B\nい と あ");
    }

    /// 値の中の `{{…}}` は、置き換えずにそのまま残す（値を入れた後にもう一度置き換えない）。
    #[test]
    fn keeps_placeholders_inside_values() {
        let filled = fill(
            "{{question}}\n{{history}}",
            &[
                ("question", "{{history}} とは？ {{"),
                ("history", "format!(\"{{}}\")"),
            ],
        );
        assert_eq!(filled.unwrap(), "{{history}} とは？ {{\nformat!(\"{{}}\")");
    }

    #[test]
    fn fails_when_placeholder_has_no_value() {
        assert_eq!(
            fill("{{a}} {{b}}", &[("a", "x")]),
            Err(TemplateError::Unfilled { name: "b".into() })
        );
    }

    #[test]
    fn fails_when_value_name_is_not_in_template() {
        assert_eq!(
            fill("{{a}}", &[("a", "x"), ("c", "y")]),
            Err(TemplateError::UnknownName { name: "c".into() })
        );
    }

    #[test]
    fn fails_when_braces_are_malformed() {
        assert_eq!(
            fill("ab {{a", &[("a", "x")]),
            Err(TemplateError::Malformed { offset: 3 })
        );
        assert_eq!(
            fill("{{ a }}", &[("a", "x")]),
            Err(TemplateError::Malformed { offset: 0 })
        );
        assert_eq!(
            fill("{{}}", &[]),
            Err(TemplateError::Malformed { offset: 0 })
        );
    }
}
