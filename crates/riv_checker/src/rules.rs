//! Rule identity: flat kebab-case names with static metadata, ty-style.
//! No alphanumeric codes — names are self-documenting in output and config.

use std::fmt;

/// The level a rule reports at. Severity is the gradual-typing ratchet:
/// `strict: true` in a pipeline promotes `unannotated-io` from Warn to Error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Ignore,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Ignore => f.write_str("ignore"),
            Level::Warn => f.write_str("warning"),
            Level::Error => f.write_str("error"),
        }
    }
}

/// Static metadata for a named rule.
#[derive(Debug)]
pub struct Rule {
    pub name: &'static str,
    pub summary: &'static str,
    pub default_level: Level,
}

macro_rules! declare_rule {
    (
        $(#[doc = $doc:literal])*
        pub static $ident:ident = {
            name: $name:literal,
            summary: $summary:literal,
            default_level: $level:expr,
        }
    ) => {
        $(#[doc = $doc])*
        pub static $ident: Rule = Rule {
            name: $name,
            summary: $summary,
            default_level: $level,
        };
    };
}

declare_rule! {
    /// A bare `riv_in()`/`riv_out()` with no schema attached. The gradual-typing
    /// warning: the call works, but the artifact is invisible to the checker.
    /// `Untyped.riv_out(...)` is the legal escape hatch and emits nothing.
    pub static UNANNOTATED_IO = {
        name: "unannotated-io",
        summary: "detects riv_in/riv_out calls without a schema",
        default_level: Level::Warn,
    }
}

declare_rule! {
    /// A step consumes an artifact that no earlier step produces. The `steps:`
    /// list is ordered and authoritative; riv does not reorder steps.
    pub static CONSUME_BEFORE_PRODUCE = {
        name: "consume-before-produce",
        summary: "detects artifacts consumed before any step produces them",
        default_level: Level::Error,
    }
}

declare_rule! {
    /// Producer and consumer of the same artifact disagree on the schema
    /// symbol. Matching is name-level in v1 (`schemas.UsersDf`).
    pub static SCHEMA_MISMATCH = {
        name: "schema-mismatch",
        summary: "detects producer/consumer schema disagreement on an artifact",
        default_level: Level::Error,
    }
}

declare_rule! {
    /// A pipeline includes itself, directly or through other pipelines.
    pub static CYCLE = {
        name: "cycle",
        summary: "detects pipelines that include themselves",
        default_level: Level::Error,
    }
}

declare_rule! {
    /// A step listed in a pipeline does not exist on disk.
    pub static MISSING_STEP = {
        name: "missing-step",
        summary: "detects steps whose file does not exist",
        default_level: Level::Error,
    }
}

declare_rule! {
    /// A pipeline YAML that cannot be read as a pipeline (bad shape, bad step
    /// kind). Untyped riv is valid riv; unreadable riv is not.
    pub static INVALID_PIPELINE = {
        name: "invalid-pipeline",
        summary: "detects malformed pipeline files",
        default_level: Level::Error,
    }
}

declare_rule! {
    /// A step's Python source does not parse. The checker cannot see its IO.
    pub static INVALID_SYNTAX = {
        name: "invalid-syntax",
        summary: "detects Python steps that fail to parse",
        default_level: Level::Error,
    }
}

/// Every rule the checker knows, for config validation and future `riv rule <name>`.
pub static REGISTRY: &[&Rule] = &[
    &UNANNOTATED_IO,
    &CONSUME_BEFORE_PRODUCE,
    &SCHEMA_MISMATCH,
    &CYCLE,
    &MISSING_STEP,
    &INVALID_PIPELINE,
    &INVALID_SYNTAX,
];
