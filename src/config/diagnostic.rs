use std::fmt;
use std::ops::Range;

/// One problem in a config file. Line and column are 1-based; columns count characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    /// A diagnostic at the start of `span` (a byte range into `text`).
    pub fn at(text: &str, span: Option<Range<usize>>, message: impl Into<String>) -> Self {
        let offset = span.map_or(0, |s| s.start).min(text.len());
        let before = &text[..offset];
        let line = before.matches('\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let column = before[line_start..].chars().count() + 1;
        Self {
            line,
            column,
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn suggest(mut self, suggestion: Option<String>) -> Self {
        self.suggestion = suggestion;
        self
    }

    /// `config.toml:3:1: message (did you mean `x`?)`
    pub fn render(&self, file: &str) -> String {
        format!("{file}:{self}")
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, " (did you mean `{suggestion}`?)")?;
        }
        Ok(())
    }
}
