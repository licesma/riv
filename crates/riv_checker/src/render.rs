//! Diagnostic rendering. v1 hand-rolls a rustc-style plain renderer with the
//! same shape `ruff_annotate_snippets` produces (header, `-->` origin,
//! underlined source lines, footers); swapping the crate in later changes
//! this module only. Checks never touch rendering.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ruff_text_size::TextRange;

use crate::diagnostic::{Annotation, Diagnostic, Severity, SubLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Full,
    Concise,
    Json,
    Github,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "full" => Ok(OutputFormat::Full),
            "concise" => Ok(OutputFormat::Concise),
            "json" => Ok(OutputFormat::Json),
            "github" => Ok(OutputFormat::Github),
            other => Err(format!(
                "unknown output format `{other}` (expected full, concise, json, or github)"
            )),
        }
    }
}

/// Caches file contents and computes line/column positions at render time.
#[derive(Default)]
struct SourceCache {
    files: HashMap<PathBuf, Option<String>>,
}

struct Position {
    /// 1-based.
    line: usize,
    /// 1-based, in characters.
    column: usize,
    line_text: String,
    /// Character offsets of the annotated range within `line_text` (clamped
    /// to the first line of the range).
    underline: (usize, usize),
}

impl SourceCache {
    fn source(&mut self, path: &Path) -> Option<&str> {
        self.files
            .entry(path.to_path_buf())
            .or_insert_with(|| fs::read_to_string(path).ok())
            .as_deref()
    }

    fn position(&mut self, path: &Path, range: TextRange) -> Option<Position> {
        let source = self.source(path)?;
        let start = usize::from(range.start()).min(source.len());
        let end = usize::from(range.end()).min(source.len());
        let line = source[..start].matches('\n').count() + 1;
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = source[start..]
            .find('\n')
            .map_or(source.len(), |i| start + i);
        let line_text = source[line_start..line_end].to_string();
        let column = source[line_start..start].chars().count() + 1;
        let underline_len = source[start..end.min(line_end)].chars().count().max(1);
        Some(Position {
            line,
            column,
            line_text,
            underline: (column - 1, underline_len),
        })
    }
}

pub fn render_to_string(diagnostics: &[Diagnostic], format: OutputFormat) -> String {
    let mut cache = SourceCache::default();
    match format {
        OutputFormat::Full => full(diagnostics, &mut cache),
        OutputFormat::Concise => concise(diagnostics, &mut cache),
        OutputFormat::Json => json(diagnostics, &mut cache),
        OutputFormat::Github => github(diagnostics, &mut cache),
    }
}

fn severity_tag(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn full(diagnostics: &[Diagnostic], cache: &mut SourceCache) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        let _ = writeln!(
            out,
            "{}[{}]: {}",
            severity_tag(diagnostic.severity),
            diagnostic.rule.name,
            diagnostic.message
        );
        let gutter = gutter_width(diagnostic, cache);
        for (i, annotation) in diagnostic.annotations.iter().enumerate() {
            render_annotation(&mut out, annotation, i == 0, gutter, cache);
        }
        for sub in &diagnostic.subs {
            let tag = match sub.level {
                SubLevel::Help => "help",
                SubLevel::Note => "note",
            };
            let _ = writeln!(out, "{:gutter$} = {tag}: {}", "", sub.message);
        }
        out.push('\n');
    }
    out
}

fn gutter_width(diagnostic: &Diagnostic, cache: &mut SourceCache) -> usize {
    diagnostic
        .annotations
        .iter()
        .filter_map(|a| {
            let range = a.span.range?;
            let pos = cache.position(&a.span.file, range)?;
            Some(pos.line.to_string().len())
        })
        .max()
        .unwrap_or(1)
        + 1
}

fn render_annotation(
    out: &mut String,
    annotation: &Annotation,
    first: bool,
    gutter: usize,
    cache: &mut SourceCache,
) {
    let file = annotation.span.file.display();
    let arrow = if first { "-->" } else { ":::" };
    let position = annotation
        .span
        .range
        .and_then(|range| cache.position(&annotation.span.file, range));
    let Some(pos) = position else {
        let _ = writeln!(out, "{:gutter$}{arrow} {file}", "");
        return;
    };
    let _ = writeln!(out, "{:gutter$}{arrow} {file}:{}:{}", "", pos.line, pos.column);
    let _ = writeln!(out, "{:gutter$} |", "");
    let _ = writeln!(out, "{:>gutter$} | {}", pos.line, pos.line_text);
    let marker = if annotation.is_primary { '^' } else { '-' };
    let (pad, len) = pos.underline;
    let _ = writeln!(
        out,
        "{:gutter$} | {}{} {}",
        "",
        " ".repeat(pad),
        marker.to_string().repeat(len),
        annotation.message.as_deref().unwrap_or_default()
    );
    let _ = writeln!(out, "{:gutter$} |", "");
}

fn concise(diagnostics: &[Diagnostic], cache: &mut SourceCache) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        let location = diagnostic
            .primary_annotation()
            .map(|a| {
                let file = a.span.file.display();
                match a
                    .span
                    .range
                    .and_then(|range| cache.position(&a.span.file, range))
                {
                    Some(pos) => format!("{file}:{}:{}", pos.line, pos.column),
                    None => file.to_string(),
                }
            })
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "{}[{}] {location}: {}",
            severity_tag(diagnostic.severity),
            diagnostic.rule.name,
            diagnostic.message
        );
    }
    out
}

fn json(diagnostics: &[Diagnostic], cache: &mut SourceCache) -> String {
    let values: Vec<serde_json::Value> = diagnostics
        .iter()
        .map(|diagnostic| {
            let annotations: Vec<serde_json::Value> = diagnostic
                .annotations
                .iter()
                .map(|a| {
                    let position = a
                        .span
                        .range
                        .and_then(|range| cache.position(&a.span.file, range));
                    serde_json::json!({
                        "file": a.span.file.display().to_string(),
                        "line": position.as_ref().map(|p| p.line),
                        "column": position.as_ref().map(|p| p.column),
                        "message": a.message,
                        "primary": a.is_primary,
                    })
                })
                .collect();
            serde_json::json!({
                "rule": diagnostic.rule.name,
                "severity": severity_tag(diagnostic.severity),
                "message": diagnostic.message,
                "annotations": annotations,
                "subs": diagnostic.subs.iter().map(|s| s.message.clone()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut out = serde_json::to_string_pretty(&values).unwrap_or_else(|_| "[]".to_string());
    out.push('\n');
    out
}

fn github(diagnostics: &[Diagnostic], cache: &mut SourceCache) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        let command = match diagnostic.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        let location = diagnostic
            .primary_annotation()
            .map(|a| {
                let file = a.span.file.display();
                match a
                    .span
                    .range
                    .and_then(|range| cache.position(&a.span.file, range))
                {
                    Some(pos) => format!("file={file},line={},col={}", pos.line, pos.column),
                    None => format!("file={file}"),
                }
            })
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "::{command} {location},title=riv ({})::{}",
            diagnostic.rule.name, diagnostic.message
        );
    }
    out
}
