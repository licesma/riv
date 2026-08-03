//! riv's static checker: parse pipelines, scan Python steps, resolve schemas,
//! and verify the artifact-passing layer between scripts. The checker is the
//! product; diagnostics quality is the UX.

pub mod checks;
pub mod diagnostic;
pub mod pipeline;
pub mod python;
pub mod render;
pub mod rules;

use std::path::{Path, PathBuf};

use diagnostic::{CheckContext, Diagnostic, Severity};
use pipeline::ResolvedStep;

/// The result of checking one or more root pipelines.
#[derive(Debug)]
pub struct CheckResult {
    /// The pipeline YAMLs that were checked, in order.
    pub pipelines: Vec<PathBuf>,
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

/// `riv check`: discover every pipeline under `root` and check each as a
/// root. Identical diagnostics arising from a pipeline being checked both
/// standalone and as a sub-pipeline are deduplicated.
pub fn check_all(root: &Path) -> CheckResult {
    let pipelines = pipeline::discover_pipelines(root);
    let mut ctx = CheckContext::new();
    for path in &pipelines {
        checks::check_root(path, &mut ctx);
    }
    let mut diagnostics = ctx.into_diagnostics();
    dedup_and_sort(&mut diagnostics);
    CheckResult {
        pipelines,
        diagnostics,
    }
}

/// `riv <pipeline> check`: check a single root pipeline.
pub fn check_pipeline(yaml: &Path) -> CheckResult {
    let mut ctx = CheckContext::new();
    checks::check_root(yaml, &mut ctx);
    let mut diagnostics = ctx.into_diagnostics();
    dedup_and_sort(&mut diagnostics);
    CheckResult {
        pipelines: vec![yaml.to_path_buf()],
        diagnostics,
    }
}

/// Resolve the ordered Python steps of a pipeline for execution. Structural
/// diagnostics found during expansion are discarded here — `run` implies
/// `check`, so the caller has already refused to start on errors.
pub fn resolve_run_steps(yaml: &Path) -> Vec<ResolvedStep> {
    let mut ctx = CheckContext::new();
    pipeline::expand(yaml, &mut ctx)
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
