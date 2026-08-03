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

- **`documentation/plan.md`** — WHAT riv is and WHY: vision, MVP,
  pipeline/schema/checker semantics, CLI verbs, gradual typing, naming
  decisions, long-term direction. Read it before any work that touches
  product behavior or semantics.
- **`documentation/STRUCTURE.md`** — HOW the repo is built: index of the four
  `documentation/structure/*.md` docs (workspace, packaging, diagnostics,
  testing) plus the distilled rules and the v1 layout tree. Read
  `STRUCTURE.md` first; open the specific `structure/*.md` only if your task
  touches that area — each is self-contained and ~6k chars by design.

Routing by task:

| Task touches… | Read |
|---|---|
| check semantics, rules, schemas, CLI behavior | `documentation/plan.md` |
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
  `documentation/structure/*.md` (or `documentation/plan.md` for semantics) in the
  same change, or doesn't happen.
