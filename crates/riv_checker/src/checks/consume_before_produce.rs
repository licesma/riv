//! `consume-before-produce`: the `steps:` list is ordered and authoritative;
//! riv does not reorder steps. A `riv_in` of an artifact that only a *later*
//! step produces is an ordering bug. An artifact no step produces is legal
//! when it already exists on disk (external input data); when it does not,
//! nothing can ever have written it and the run is doomed.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::diagnostic::{CheckContext, Span};
use crate::python::Direction;
use crate::rules::CONSUME_BEFORE_PRODUCE;

use super::AnalyzedStep;

pub(crate) fn check(steps: &[AnalyzedStep], ctx: &mut CheckContext) {
    // Every producer in the whole river, so late producers can be named
    // in the diagnostic.
    let mut all_producers: HashMap<PathBuf, &AnalyzedStep> = HashMap::new();
    for step in steps {
        for call in &step.file.calls {
            if call.direction == Direction::Out
                && let Some(path) = &call.path
            {
                all_producers.entry(step.artifact_key(path)).or_insert(step);
            }
        }
    }

    let mut produced: HashSet<PathBuf> = HashSet::new();
    for step in steps {
        // Calls appear in source order: a script may read its own earlier output.
        for call in &step.file.calls {
            let Some(path) = &call.path else { continue };
            let key = step.artifact_key(path);
            match call.direction {
                Direction::Out => {
                    produced.insert(key);
                }
                Direction::In => {
                    if produced.contains(&key) || key.is_file() {
                        continue;
                    }
                    let Some(builder) = ctx.report(&CONSUME_BEFORE_PRODUCE, step.step.strict)
                    else {
                        continue;
                    };
                    let builder = builder
                        .message(format!("`{path}` is consumed before it is produced"))
                        .primary(
                            Span::new(&step.file.path, call.call_range),
                            format!("`{path}` does not exist yet"),
                        );
                    let builder = if let Some(producer) = all_producers.get(&key) {
                        builder.note(format!(
                            "`{}` produces it, but runs later — steps run in the order listed; riv does not reorder them",
                            producer.file.path.display()
                        ))
                    } else {
                        builder.note(
                            "no step in this river produces it, and it does not exist on disk",
                        )
                    };
                    builder.emit();
                }
            }
        }
    }
}
