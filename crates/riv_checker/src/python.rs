//! Python analysis: parse a step with `ruff_python_parser` and extract every
//! `riv_in`/`riv_out` call together with its schema symbol and artifact path.
//!
//! Resolution is name-level and AST-local, per the MVP: the schema is the
//! imported (or locally defined) class name attached to the call, qualified
//! by the module it was imported from.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::diagnostic::{CheckContext, Span};
use crate::rules;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    pub fn function_name(self) -> &'static str {
        match self {
            Direction::In => "riv_in",
            Direction::Out => "riv_out",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaRef {
    /// Bare `riv_in(...)` / `riv_out(...)` — the gradual-typing warning.
    Unannotated,
    /// `riv_in[Untyped](...)` — the explicit escape hatch; emits nothing.
    Untyped,
    /// `riv_in[UsersDf](...)` — qualified symbol, e.g. `schemas.UsersDf`.
    Named(String),
}

#[derive(Debug, Clone)]
pub struct IoCall {
    pub direction: Direction,
    pub schema: SchemaRef,
    /// The artifact path, when it is a plain string literal. Dynamic paths
    /// are invisible to the checker in v1.
    pub path: Option<String>,
    pub call_range: TextRange,
    /// Range of the schema expression (`UsersDf` in `riv_in[UsersDf](...)`).
    pub schema_range: Option<TextRange>,
}

#[derive(Debug)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub source: String,
    pub calls: Vec<IoCall>,
}

#[derive(Debug, Clone)]
enum Binding {
    /// `from riv import riv_in` (possibly aliased).
    RivFunc(Direction),
    /// `import riv` (possibly aliased).
    RivModule,
    /// `from riv import Untyped`.
    Untyped,
    /// `from riv import Schema`.
    SchemaBase,
    /// `from M import N` -> qualified `M.N`.
    Imported(String),
    /// `class N(Schema)` in this file -> qualified `<stem>.N`.
    LocalClass(String),
}

pub fn scan(path: &Path, ctx: &mut CheckContext, strict: bool) -> Option<ScannedFile> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            if let Some(b) = ctx.report(&rules::INVALID_SYNTAX, strict) {
                b.message(format!("cannot read `{}`: {err}", path.display()))
                    .primary(Span::file_only(path), "unreadable step")
                    .emit();
            }
            return None;
        }
    };
    let parsed = match ruff_python_parser::parse_module(&source) {
        Ok(parsed) => parsed,
        Err(err) => {
            if let Some(b) = ctx.report(&rules::INVALID_SYNTAX, strict) {
                b.message(format!("syntax error: {}", err.error))
                    .primary(Span::new(path, err.location), "cannot parse this step")
                    .emit();
            }
            return None;
        }
    };

    let module_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut visitor = ScanVisitor {
        module_stem,
        bindings: HashMap::new(),
        calls: Vec::new(),
    };
    visitor.visit_body(&parsed.syntax().body);

    Some(ScannedFile {
        path: path.to_path_buf(),
        source,
        calls: visitor.calls,
    })
}

struct ScanVisitor {
    module_stem: String,
    bindings: HashMap<String, Binding>,
    calls: Vec<IoCall>,
}

impl ScanVisitor {
    fn bind_riv_member(&mut self, member: &str, bound_as: &str) {
        let binding = match member {
            "riv_in" => Binding::RivFunc(Direction::In),
            "riv_out" => Binding::RivFunc(Direction::Out),
            "Untyped" => Binding::Untyped,
            "Schema" => Binding::SchemaBase,
            _ => return,
        };
        self.bindings.insert(bound_as.to_string(), binding);
    }

    fn record_import_from(&mut self, import: &ast::StmtImportFrom) {
        let Some(module) = import.module.as_ref() else {
            return;
        };
        let module = module.as_str();
        for alias in &import.names {
            let name = alias.name.as_str();
            let bound_as = alias
                .asname
                .as_ref()
                .map_or(name, ruff_python_ast::Identifier::as_str);
            if module == "riv" {
                if name == "*" {
                    for member in ["riv_in", "riv_out", "Untyped", "Schema"] {
                        self.bind_riv_member(member, member);
                    }
                } else {
                    self.bind_riv_member(name, bound_as);
                }
            } else if name != "*" {
                self.bindings.insert(
                    bound_as.to_string(),
                    Binding::Imported(format!("{module}.{name}")),
                );
            }
        }
    }

    fn record_import(&mut self, import: &ast::StmtImport) {
        for alias in &import.names {
            if alias.name.as_str() == "riv" {
                let bound_as = alias
                    .asname
                    .as_ref()
                    .map_or("riv", ruff_python_ast::Identifier::as_str);
                self.bindings
                    .insert(bound_as.to_string(), Binding::RivModule);
            }
        }
    }

    fn record_class_def(&mut self, class: &ast::StmtClassDef) {
        let is_schema = class.bases().iter().any(|base| self.is_schema_base(base));
        if is_schema {
            let name = class.name.as_str();
            self.bindings.insert(
                name.to_string(),
                Binding::LocalClass(format!("{}.{name}", self.module_stem)),
            );
        }
    }

    fn is_schema_base(&self, base: &Expr) -> bool {
        match base {
            Expr::Name(name) => matches!(
                self.bindings.get(name.id.as_str()),
                Some(Binding::SchemaBase | Binding::Untyped | Binding::LocalClass(_))
            ),
            // `riv.Schema` / `riv.Untyped`
            Expr::Attribute(attr) => {
                matches!(attr.value.as_ref(), Expr::Name(n)
                    if matches!(self.bindings.get(n.id.as_str()), Some(Binding::RivModule)))
                    && matches!(attr.attr.as_str(), "Schema" | "Untyped")
            }
            _ => false,
        }
    }

    /// Resolve `riv_in` / `riv.riv_in` (possibly aliased) to a direction.
    fn resolve_riv_func(&self, expr: &Expr) -> Option<Direction> {
        match expr {
            Expr::Name(name) => match self.bindings.get(name.id.as_str()) {
                Some(Binding::RivFunc(direction)) => Some(*direction),
                _ => None,
            },
            Expr::Attribute(attr) => {
                if matches!(attr.value.as_ref(), Expr::Name(n)
                    if matches!(self.bindings.get(n.id.as_str()), Some(Binding::RivModule)))
                {
                    match attr.attr.as_str() {
                        "riv_in" => Some(Direction::In),
                        "riv_out" => Some(Direction::Out),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Resolve the subscript of `riv_in[<schema>](...)` to a schema reference.
    fn resolve_schema_expr(&self, expr: &Expr) -> Option<SchemaRef> {
        match expr {
            Expr::Name(name) => match self.bindings.get(name.id.as_str()) {
                Some(Binding::Untyped) => Some(SchemaRef::Untyped),
                Some(Binding::Imported(qualified) | Binding::LocalClass(qualified)) => {
                    Some(SchemaRef::Named(qualified.clone()))
                }
                // Best effort: an unresolvable name still names a schema.
                _ => Some(SchemaRef::Named(name.id.to_string())),
            },
            Expr::Attribute(attr) => {
                // `riv_in[riv.Untyped](...)`
                if let Expr::Name(n) = attr.value.as_ref()
                    && matches!(self.bindings.get(n.id.as_str()), Some(Binding::RivModule))
                    && attr.attr.as_str() == "Untyped"
                {
                    return Some(SchemaRef::Untyped);
                }
                dotted_name(expr).map(SchemaRef::Named)
            }
            _ => None,
        }
    }

    fn record_call(&mut self, call: &ast::ExprCall) {
        let (direction, schema, schema_range) = match call.func.as_ref() {
            // `riv_in[UsersDf](...)` / `riv.riv_in[UsersDf](...)`
            Expr::Subscript(sub) => {
                let Some(direction) = self.resolve_riv_func(sub.value.as_ref()) else {
                    return;
                };
                let Some(schema) = self.resolve_schema_expr(sub.slice.as_ref()) else {
                    return;
                };
                (direction, schema, Some(sub.slice.range()))
            }
            // Bare `riv_in(...)` / `riv.riv_in(...)`: unannotated.
            func => match self.resolve_riv_func(func) {
                Some(direction) => (direction, SchemaRef::Unannotated, None),
                None => return,
            },
        };

        let path_index = match direction {
            Direction::In => 0,
            Direction::Out => 1,
        };
        let path = call
            .arguments
            .args
            .get(path_index)
            .and_then(|arg| match arg {
                Expr::StringLiteral(lit) => Some(lit.value.to_str().to_string()),
                _ => None,
            });

        self.calls.push(IoCall {
            direction,
            schema,
            path,
            call_range: call.range(),
            schema_range,
        });
    }
}

impl Visitor<'_> for ScanVisitor {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ImportFrom(import) => self.record_import_from(import),
            Stmt::Import(import) => self.record_import(import),
            Stmt::ClassDef(class) => self.record_class_def(class),
            _ => {}
        }
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            self.record_call(call);
        }
        walk_expr(self, expr);
    }
}

/// Render `a.b.c` from a Name/Attribute chain.
fn dotted_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Attribute(attr) => {
            let base = dotted_name(attr.value.as_ref())?;
            Some(format!("{base}.{}", attr.attr.as_str()))
        }
        _ => None,
    }
}
