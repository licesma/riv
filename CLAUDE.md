# riv

Typed data pipelines for Python: a Rust static checker/CLI (`riv check`,
`riv <pipeline> run`) plus a tiny pure-Python runtime (`riv_in`, `riv_out`,
`Schema`). TypeScript is the north star: annotations are erased at runtime,
checking is gradual, untyped riv is valid riv forever.

## Where context lives — read before working

Design and alignment docs live in **`dev_docs/`**. They are for
communication between Esteban and coding agents only: the folder is
gitignored and must NEVER be committed, pushed, or referenced from
tracked files — not even in git history.

Not to be confused with **`documentation/`**, which is tracked,
user-facing documentation linked from the README. It must never
reference `dev_docs/`.

- **`dev_docs/plan.md`** — WHAT riv is and WHY: index of the seven
  `dev_docs/plan/*.md` docs (vision, adoption, pipelines, inputs,
  checking, execution, tracing). Read the index first; open the specific `plan/*.md`
  your task touches — each is self-contained by design.
- **`dev_docs/STRUCTURE.md`** — HOW the repo is built: index of the four
  `dev_docs/structure/*.md` docs (workspace, packaging, diagnostics,
  testing) plus the distilled rules and the v1 layout tree. Same pattern:
  index first, then the one doc your task touches.

Routing by task:

| Task touches… | Read |
|---|---|
| vision, philosophy, long-term direction | `dev_docs/plan/vision.md` |
| MVP path, runtime API, schemas, formats | `dev_docs/plan/adoption.md` |
| pipeline YAML, steps, scripts vs functions | `dev_docs/plan/pipelines.md` |
| params, with:, defs, CLI flags, river input: | `dev_docs/plan/inputs.md` |
| check semantics, gradual typing, strictness | `dev_docs/plan/checking.md` |
| CLI verbs, run, environments, uv, callables | `dev_docs/plan/execution.md` |
| impact analysis, blast radius, provenance | `dev_docs/plan/tracing.md` |
| creating/moving crates, deps, Cargo.toml | `dev_docs/structure/workspace.md` |
| pyproject, wheel, python/riv layout, versions | `dev_docs/structure/packaging.md` |
| diagnostic types, rule names, severity, output | `dev_docs/structure/diagnostics.md` |
| tests, fixtures, snapshots, examples/ | `dev_docs/structure/testing.md` |

## Commits

All commits are authored solely by Esteban <licesma@gmail.com>. Claude gets
no credit whatsoever: no `Co-Authored-By` trailer, no "Generated with" lines,
no mention of Claude or AI anywhere in commit messages or PR bodies.

## Invariants (apply to every change)

- The static checker is the product; diagnostics quality is the UX.
- The Python runtime stays tiny and pure Python — orchestration logic never
  leaks into it; zero runtime dependencies.
- Untyped riv is valid riv: never turn the unannotated path into an error by
  default; `Untyped` is a legal decision, not a suppression.
- Docs are law: a structural change updates the matching
  `dev_docs/structure/*.md` (or `dev_docs/plan/*.md` for semantics)
  in the same change, or doesn't happen.
