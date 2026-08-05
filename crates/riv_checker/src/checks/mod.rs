//! Checks are modules, never crates. Each check is a plain function over the
//! resolved, scanned river that emits through the [`CheckContext`].

mod consume_before_produce;
mod schema_mismatch;
mod unannotated_io;

use std::path::{Path, PathBuf};

use crate::diagnostic::CheckContext;
use crate::python::{self, ScannedFile};
use crate::river::{self, ResolvedStep};

/// A river step joined with the scan of its Python source.
pub(crate) struct AnalyzedStep {
    pub(crate) step: ResolvedStep,
    pub(crate) file: ScannedFile,
}

impl AnalyzedStep {
    /// The identity of an artifact named `path` inside this step: its
    /// path-literal resolved against the river directory the step runs in.
    pub(crate) fn artifact_key(&self, path: &str) -> PathBuf {
        river::normalize(&self.step.dir.join(path))
    }
}

/// Check one root river: expand it (cycles, missing steps), scan every
/// Python step (syntax), then run the artifact checks over the ordered steps.
pub fn check_root(root_yaml: &Path, ctx: &mut CheckContext) {
    let steps = river::expand(root_yaml, ctx);
    let analyzed: Vec<AnalyzedStep> = steps
        .into_iter()
        .filter_map(|step| {
            python::scan(&step.script, ctx, step.strict).map(|file| AnalyzedStep { step, file })
        })
        .collect();

    unannotated_io::check(&analyzed, ctx);
    consume_before_produce::check(&analyzed, ctx);
    schema_mismatch::check(&analyzed, ctx);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::diagnostic::CheckContext;
    use crate::render::{OutputFormat, render_to_string};

    /// Check a fixture project (a directory under `resources/test/fixtures`
    /// containing at least one river YAML) and render its diagnostics.
    fn check_fixture(fixture: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/fixtures")
            .join(fixture);
        let cwd = std::env::current_dir().unwrap();
        // Diagnostics carry paths as given; check relative to the fixture so
        // snapshots are machine-independent.
        let rivers = crate::river::discover_rivers(&root);
        let mut ctx = CheckContext::new();
        for river in &rivers {
            let relative = river.strip_prefix(&cwd).unwrap_or(river);
            super::check_root(relative, &mut ctx);
        }
        let mut diagnostics = ctx.into_diagnostics();
        crate::dedup_and_sort(&mut diagnostics);
        let rendered = render_to_string(&diagnostics, OutputFormat::Full);
        // Strip the absolute fixture prefix out of rendered paths.
        rendered.replace(&format!("{}/", root.display()), "")
    }

    macro_rules! assert_diagnostics {
        ($fixture:literal) => {
            insta::with_settings!({
                snapshot_path => "snapshots",
                prepend_module_to_snapshot => false,
                snapshot_suffix => "",
            }, {
                insta::assert_snapshot!(
                    $fixture.replace('/', "__"),
                    check_fixture($fixture)
                );
            });
        };
    }

    #[test]
    fn unannotated_io_basic() {
        assert_diagnostics!("unannotated-io/basic");
    }

    #[test]
    fn unannotated_io_strict() {
        assert_diagnostics!("unannotated-io/strict");
    }

    #[test]
    fn unannotated_io_untyped_escape_hatch() {
        assert_diagnostics!("unannotated-io/untyped-escape-hatch");
    }

    #[test]
    fn consume_before_produce_basic() {
        assert_diagnostics!("consume-before-produce/basic");
    }

    #[test]
    fn schema_mismatch_basic() {
        assert_diagnostics!("schema-mismatch/basic");
    }

    #[test]
    fn cycle_basic() {
        assert_diagnostics!("cycle/basic");
    }

    #[test]
    fn missing_step_basic() {
        assert_diagnostics!("missing-step/basic");
    }

    #[test]
    fn clean_river_has_no_diagnostics() {
        assert_diagnostics!("clean/basic");
    }
}
