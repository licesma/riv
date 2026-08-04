//! `unannotated-io`: a bare `riv_in()`/`riv_out()` with no schema attached.
//! Warn by default; `strict: true` promotes it to an error. `riv_*[Untyped]`
//! emits nothing — a decision, not a suppression.

use crate::diagnostic::{CheckContext, Span};
use crate::python::{Direction, SchemaRef};
use crate::rules::UNANNOTATED_IO;

use super::AnalyzedStep;

pub(crate) fn check(steps: &[AnalyzedStep], ctx: &mut CheckContext) {
    for step in steps {
        for call in &step.file.calls {
            if call.schema != SchemaRef::Unannotated {
                continue;
            }
            let Some(builder) = ctx.report(&UNANNOTATED_IO, step.step.strict) else {
                continue;
            };
            let what = match &call.path {
                Some(path) => format!("artifact `{path}`"),
                None => "this artifact".to_string(),
            };
            let (verb, example) = match call.direction {
                Direction::In => ("consumed", "df = riv_in[UsersDf](path)"),
                Direction::Out => ("produced", "riv_out[UsersDf](df, path)"),
            };
            builder
                .message(format!(
                    "{what} is {verb} without a schema; the checker cannot see it"
                ))
                .primary(
                    Span::new(&step.file.path, call.call_range),
                    format!("bare `{}()`", call.direction.function_name()),
                )
                .help(format!(
                    "attach a schema (`{example}`) or mark the decision with `{}[Untyped](...)`",
                    call.direction.function_name()
                ))
                .emit();
        }
    }
}
