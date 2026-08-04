# riv

Typed data pipelines for Python: a Rust static checker/CLI (`riv check`,
`riv <pipeline> run`) plus a tiny pure-Python runtime (`riv_in`, `riv_out`,
`Schema`). TypeScript is the north star: annotations are erased at runtime,
checking is gradual, untyped riv is valid riv forever.

## Where context lives — read before working

Design and alignment docs live in **`documentation/`**. They are for
communication between Esteban and coding agents only: the folder is
gitignored and must NEVER be committed, pushed, or referenced from
tracked files — not even in git history.

- **`documentation/plan.md`** — WHAT riv is and WHY: index of the six
  `documentation/plan/*.md` docs (vision, adoption, pipelines, checking,
  execution, tracing). Read the index first; open the specific `plan/*.md`
  your task touches — each is self-contained by design.
- **`documentation/STRUCTURE.md`** — HOW the repo is built: index of the four
  `documentation/structure/*.md` docs (workspace, packaging, diagnostics,
  testing) plus the distilled rules and the v1 layout tree. Same pattern:
  index first, then the one doc your task touches.

Routing by task:

| Task touches… | Read |
|---|---|
| vision, philosophy, long-term direction | `documentation/plan/vision.md` |
| MVP path, runtime API, schemas, formats | `documentation/plan/adoption.md` |
| pipeline YAML, steps, scripts vs functions | `documentation/plan/pipelines.md` |
| check semantics, gradual typing, strictness | `documentation/plan/checking.md` |
| CLI verbs, run, environments, uv, callables | `documentation/plan/execution.md` |
| impact analysis, blast radius, provenance | `documentation/plan/tracing.md` |
| creating/moving crates, deps, Cargo.toml | `documentation/structure/workspace.md` |
| pyproject, wheel, python/riv layout, versions | `documentation/structure/packaging.md` |
| diagnostic types, rule names, severity, output | `documentation/structure/diagnostics.md` |
| tests, fixtures, snapshots, examples/ | `documentation/structure/testing.md` |

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
  `documentation/structure/*.md` (or `documentation/plan/*.md` for semantics)
  in the same change, or doesn't happen.
