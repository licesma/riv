//! The diagnostic data model, shaped after `ruff_db`'s `DiagnosticInner`
//! (copied in shape, never depended on): named rule, severity, message,
//! primary/secondary annotations, sub-diagnostics. Rendering lives in
//! [`crate::render`]; checks never touch it.

use std::path::PathBuf;

use ruff_text_size::TextRange;

use crate::rules::{Level, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

/// A file plus an optional byte range inside it. Line/column are computed
/// only at render time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub file: PathBuf,
    pub range: Option<TextRange>,
}

impl Span {
    pub fn new(file: impl Into<PathBuf>, range: TextRange) -> Self {
        Span {
            file: file.into(),
            range: Some(range),
        }
    }

    pub fn file_only(file: impl Into<PathBuf>) -> Self {
        Span {
            file: file.into(),
            range: None,
        }
    }
}

/// One source pointer. Exactly one annotation per diagnostic is primary;
/// secondaries add context — riv's flagship UX is the cross-file secondary
/// (primary at the consuming `riv_in`, secondary at the producing `riv_out`
/// or the pipeline YAML step).
#[derive(Debug, Clone)]
pub struct Annotation {
    pub span: Span,
    pub message: Option<String>,
    pub is_primary: bool,
}

impl Annotation {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Annotation {
            span,
            message: Some(message.into()),
            is_primary: true,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Annotation {
            span,
            message: Some(message.into()),
            is_primary: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubLevel {
    Help,
    Note,
}

/// Rendered as a footer line under the snippet(s).
#[derive(Debug, Clone)]
pub struct SubDiagnostic {
    pub level: SubLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub rule: &'static Rule,
    pub severity: Severity,
    pub message: String,
    pub annotations: Vec<Annotation>,
    pub subs: Vec<SubDiagnostic>,
}

impl Diagnostic {
    pub fn primary_annotation(&self) -> Option<&Annotation> {
        self.annotations
            .iter()
            .find(|a| a.is_primary)
            .or(self.annotations.first())
    }
}

/// Collects diagnostics during a check run and applies the severity ratchet.
/// Checks are plain functions over the resolved pipeline that call
/// [`CheckContext::report`]; `None` means the rule is ignored at this site.
#[derive(Debug, Default)]
pub struct CheckContext {
    diagnostics: Vec<Diagnostic>,
}

impl CheckContext {
    pub fn new() -> Self {
        CheckContext::default()
    }

    /// Start a diagnostic for `rule` at the effective level for a step.
    /// `strict` is the ratchet: it promotes `unannotated-io` to an error.
    pub fn report(&mut self, rule: &'static Rule, strict: bool) -> Option<DiagnosticBuilder<'_>> {
        let level = effective_level(rule, strict);
        let severity = match level {
            Level::Ignore => return None,
            Level::Warn => Severity::Warning,
            Level::Error => Severity::Error,
        };
        Some(DiagnosticBuilder {
            ctx: self,
            diagnostic: Diagnostic {
                rule,
                severity,
                message: String::new(),
                annotations: Vec::new(),
                subs: Vec::new(),
            },
        })
    }

    pub fn extend(&mut self, other: CheckContext) {
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

pub fn effective_level(rule: &Rule, strict: bool) -> Level {
    if strict && rule.default_level == Level::Warn {
        Level::Error
    } else {
        rule.default_level
    }
}

/// Builds a diagnostic and files it into the context when finished.
pub struct DiagnosticBuilder<'a> {
    ctx: &'a mut CheckContext,
    diagnostic: Diagnostic,
}

impl DiagnosticBuilder<'_> {
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.diagnostic.message = message.into();
        self
    }

    #[must_use]
    pub fn primary(mut self, span: Span, label: impl Into<String>) -> Self {
        self.diagnostic
            .annotations
            .push(Annotation::primary(span, label));
        self
    }

    #[must_use]
    pub fn secondary(mut self, span: Span, label: impl Into<String>) -> Self {
        self.diagnostic
            .annotations
            .push(Annotation::secondary(span, label));
        self
    }

    #[must_use]
    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.diagnostic.subs.push(SubDiagnostic {
            level: SubLevel::Help,
            message: message.into(),
        });
        self
    }

    #[must_use]
    pub fn note(mut self, message: impl Into<String>) -> Self {
        self.diagnostic.subs.push(SubDiagnostic {
            level: SubLevel::Note,
            message: message.into(),
        });
        self
    }

    pub fn emit(self) {
        self.ctx.diagnostics.push(self.diagnostic);
    }
}
