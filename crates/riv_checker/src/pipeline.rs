//! Pipeline YAML model: loading, discovery, and recursive expansion into an
//! ordered list of Python steps. Cycle and missing-step problems are found
//! here during expansion and reported through the check context.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use ruff_text_size::{TextRange, TextSize};
use serde::Deserialize;

use crate::diagnostic::{CheckContext, Span};
use crate::rules;

/// One pipeline YAML document, as written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineDoc {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub strict: bool,
    pub steps: Vec<String>,
}

/// A loaded pipeline file: the parsed document plus enough source context to
/// point diagnostics at individual step lines.
#[derive(Debug, Clone)]
pub struct PipelineFile {
    pub path: PathBuf,
    pub source: String,
    pub doc: PipelineDoc,
}

impl PipelineFile {
    pub fn load(path: &Path, ctx: &mut CheckContext) -> Option<PipelineFile> {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(err) => {
                if let Some(b) = ctx.report(&rules::INVALID_PIPELINE, false) {
                    b.message(format!("cannot read `{}`: {err}", path.display()))
                        .primary(Span::file_only(path), "unreadable pipeline")
                        .emit();
                }
                return None;
            }
        };
        match serde_yaml::from_str::<PipelineDoc>(&source) {
            Ok(doc) => Some(PipelineFile {
                path: path.to_path_buf(),
                source,
                doc,
            }),
            Err(err) => {
                if let Some(b) = ctx.report(&rules::INVALID_PIPELINE, false) {
                    b.message(format!("`{}` is not a valid pipeline: {err}", path.display()))
                        .primary(Span::file_only(path), "invalid pipeline file")
                        .emit();
                }
                None
            }
        }
    }

    /// Byte range of `step` as written in the YAML source, for annotations.
    fn step_range(&self, step: &str) -> Option<TextRange> {
        let start = self.source.find(step)?;
        let start = TextSize::try_from(start).ok()?;
        let len = TextSize::try_from(step.len()).ok()?;
        Some(TextRange::at(start, len))
    }

    fn step_span(&self, step: &str) -> Span {
        match self.step_range(step) {
            Some(range) => Span::new(&self.path, range),
            None => Span::file_only(&self.path),
        }
    }
}

/// A Python step after recursive expansion, in execution order.
#[derive(Debug, Clone)]
pub struct ResolvedStep {
    /// The `.py` file to run.
    pub script: PathBuf,
    /// Directory of the pipeline YAML that listed this step; scripts run with
    /// this as their working directory, and artifact paths resolve against it.
    pub dir: PathBuf,
    /// Effective strictness: OR of `strict:` along the inclusion chain.
    pub strict: bool,
}

/// Is this yaml file shaped like a pipeline (a mapping with a `steps` list)?
/// Non-pipeline YAML (CI configs, etc.) is silently skipped by discovery.
fn looks_like_pipeline(source: &str) -> bool {
    matches!(
        serde_yaml::from_str::<serde_yaml::Value>(source),
        Ok(serde_yaml::Value::Mapping(m)) if matches!(m.get("steps"), Some(serde_yaml::Value::Sequence(_)))
    )
}

const SKIP_DIRS: &[&str] = &["target", "node_modules", "__pycache__", "venv"];

/// Find every pipeline YAML under `root`, sorted for deterministic output.
pub fn discover_pipelines(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()) {
                    continue;
                }
                stack.push(path);
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yaml" | "yml")
            ) && let Ok(source) = fs::read_to_string(&path)
                && looks_like_pipeline(&source)
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Lexically normalize a path (resolve `.` and `..`) without touching the
/// filesystem, so the same artifact written and read via different relative
/// spellings gets one identity.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Recursively expand a root pipeline into ordered Python steps, reporting
/// cycles, missing steps, and unknown step kinds along the way.
pub fn expand(root: &Path, ctx: &mut CheckContext) -> Vec<ResolvedStep> {
    let mut steps = Vec::new();
    let mut in_progress = HashSet::new();
    expand_into(root, false, &mut in_progress, &mut steps, ctx);
    steps
}

fn expand_into(
    yaml_path: &Path,
    inherited_strict: bool,
    in_progress: &mut HashSet<PathBuf>,
    out: &mut Vec<ResolvedStep>,
    ctx: &mut CheckContext,
) {
    let yaml_path = normalize(yaml_path);
    let Some(pipeline) = PipelineFile::load(&yaml_path, ctx) else {
        return;
    };
    in_progress.insert(yaml_path.clone());
    let dir = yaml_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let strict = inherited_strict || pipeline.doc.strict;

    for step in &pipeline.doc.steps {
        let step_path = normalize(&dir.join(step));
        match step_path.extension().and_then(|e| e.to_str()) {
            Some("yaml" | "yml") => {
                if in_progress.contains(&step_path) {
                    if let Some(b) = ctx.report(&rules::CYCLE, strict) {
                        b.message(format!(
                            "pipeline `{}` includes itself via `{step}`",
                            step_path.display()
                        ))
                        .primary(pipeline.step_span(step), "this step closes the cycle")
                        .emit();
                    }
                    continue;
                }
                if !step_path.is_file() {
                    report_missing(&pipeline, step, &step_path, strict, ctx);
                    continue;
                }
                expand_into(&step_path, strict, in_progress, out, ctx);
            }
            Some("py") => {
                if !step_path.is_file() {
                    report_missing(&pipeline, step, &step_path, strict, ctx);
                    continue;
                }
                out.push(ResolvedStep {
                    script: step_path,
                    dir: dir.clone(),
                    strict,
                });
            }
            _ => {
                if let Some(b) = ctx.report(&rules::INVALID_PIPELINE, strict) {
                    b.message(format!(
                        "step `{step}` is not a `.py` script or a `.yaml` pipeline"
                    ))
                    .primary(pipeline.step_span(step), "unsupported step kind")
                    .note("steps are scripts in v1: a Python file or another pipeline")
                    .emit();
                }
            }
        }
    }
    in_progress.remove(&yaml_path);
}

fn report_missing(
    pipeline: &PipelineFile,
    step: &str,
    step_path: &Path,
    strict: bool,
    ctx: &mut CheckContext,
) {
    if let Some(b) = ctx.report(&rules::MISSING_STEP, strict) {
        b.message(format!("step `{}` does not exist", step_path.display()))
            .primary(pipeline.step_span(step), "listed here")
            .emit();
    }
}
