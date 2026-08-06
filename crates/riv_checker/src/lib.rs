//! riv's static checker: parse rivers, scan Python steps, resolve schemas,
//! and verify the artifact-passing layer between scripts. The checker is the
//! product; diagnostics quality is the UX.

pub mod checks;
pub mod diagnostic;
pub mod python;
pub mod render;
pub mod river;
pub mod rules;

use std::path::{Path, PathBuf};

use diagnostic::{CheckContext, Diagnostic, Severity};
use river::ResolvedStep;

/// The result of checking one or more root rivers.
#[derive(Debug)]
pub struct CheckResult {
    /// The river YAMLs that were checked, in order.
    pub rivers: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }
}

/// `riv check -r`: discover every river under `root` and check each as a
/// root. Identical diagnostics arising from a river being checked both
/// standalone and as a sub-river are deduplicated.
pub fn check_all(root: &Path) -> CheckResult {
    check_rivers(river::discover_rivers(root))
}

/// Discover every river YAML under `root` (for `-r` over explicit dirs).
pub fn discover_rivers(root: &Path) -> Vec<PathBuf> {
    river::discover_rivers(root)
}

/// Check the given root rivers, deduplicating identical diagnostics.
pub fn check_rivers(rivers: Vec<PathBuf>) -> CheckResult {
    let mut ctx = CheckContext::new();
    for path in &rivers {
        checks::check_root(path, &mut ctx);
    }
    let mut diagnostics = ctx.into_diagnostics();
    dedup_and_sort(&mut diagnostics);
    CheckResult {
        rivers,
        diagnostics,
    }
}

/// Check a single root river.
pub fn check_river(yaml: &Path) -> CheckResult {
    check_rivers(vec![yaml.to_path_buf()])
}

/// Resolve the ordered Python steps of a river for execution. Structural
/// diagnostics found during expansion are discarded here — `run` implies
/// `check`, so the caller has already refused to start on errors.
pub fn resolve_run_steps(yaml: &Path) -> Vec<ResolvedStep> {
    let mut ctx = CheckContext::new();
    river::expand(yaml, &mut ctx)
}

pub(crate) fn dedup_and_sort(diagnostics: &mut Vec<Diagnostic>) {
    let sort_key = |d: &Diagnostic| {
        let (file, offset) = d
            .primary_annotation()
            .map(|a| {
                (
                    a.span.file.clone(),
                    a.span.range.map_or(0, |r| u32::from(r.start())),
                )
            })
            .unwrap_or_default();
        (file, offset, d.rule.name, d.message.clone())
    };
    diagnostics.sort_by_key(sort_key);
    diagnostics.dedup_by_key(|d| sort_key(d));
}
