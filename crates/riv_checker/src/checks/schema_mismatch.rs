//! `schema-mismatch`: producer and consumer of the same artifact must agree
//! on the schema symbol. Matching is name-level in v1. `Untyped` and bare
//! (unannotated) calls match anything — the former is a decision, the latter
//! is already reported by `unannotated-io`.

use std::collections::HashMap;
use std::path::PathBuf;

use ruff_text_size::TextRange;

use crate::diagnostic::{CheckContext, Span};
use crate::python::{Direction, SchemaRef};
use crate::rules::SCHEMA_MISMATCH;

use super::AnalyzedStep;

struct Producer<'a> {
    schema: &'a str,
    file: &'a std::path::Path,
    range: TextRange,
}

pub(crate) fn check(steps: &[AnalyzedStep], ctx: &mut CheckContext) {
    // Most recent typed producer of each artifact, in step order.
    let mut producers: HashMap<PathBuf, Producer> = HashMap::new();

    for step in steps {
        for call in &step.file.calls {
            let Some(path) = &call.path else { continue };
            let key = step.artifact_key(path);
            match call.direction {
                Direction::Out => {
                    if let SchemaRef::Named(schema) = &call.schema {
                        producers.insert(
                            key,
                            Producer {
                                schema,
                                file: &step.file.path,
                                range: call.schema_range.unwrap_or(call.call_range),
                            },
                        );
                    }
                }
                Direction::In => {
                    let SchemaRef::Named(consumer_schema) = &call.schema else {
                        continue;
                    };
                    let Some(producer) = producers.get(&key) else {
                        continue;
                    };
                    if producer.schema == consumer_schema {
                        continue;
                    }
                    let Some(builder) = ctx.report(&SCHEMA_MISMATCH, step.step.strict) else {
                        continue;
                    };
                    builder
                        .message(format!(
                            "`{path}` is consumed as `{consumer_schema}` but was produced as `{}`",
                            producer.schema
                        ))
                        .primary(
                            Span::new(&step.file.path, call.schema_range.unwrap_or(call.call_range)),
                            format!("consumed as `{consumer_schema}`"),
                        )
                        .secondary(
                            Span::new(producer.file, producer.range),
                            format!("produced as `{}`", producer.schema),
                        )
                        .note("schema matching is name-level: producer and consumer must use the same schema symbol")
                        .emit();
                }
            }
        }
    }
}
