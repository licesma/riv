# riv

Typed data pipelines for Python: a fast static checker (`riv check`) plus a
tiny, dependency-free runtime (`riv_in`, `riv_out`, `Schema`).

The model is TypeScript's: schemas are erased at runtime, checking is gradual,
and untyped riv is valid riv — forever. `riv check` plays the role of `tsc`
for the artifact-passing layer between your scripts.

## Quickstart

```bash
pip install riv
```

Replace your pickle round-trips with the untyped form (works immediately,
warns under `riv check` — the full walkthrough is
[zero-cost adoption](https://github.com/licesma/riv/blob/main/documentation/zero-cost-adoption.md)):

```python
from riv import riv_in, riv_out

riv_out(df, "users.pkl")
df = riv_in("users.pkl")
```

Then type it by attaching a schema:

```python
# schemas.py
from riv import Schema

class UsersDf(Schema): ...
```

```python
from schemas import UsersDf

UsersDf.riv_out(df, "users.pkl")
df = UsersDf.riv_in("users.pkl")
```

Declare a pipeline (steps are ordered; a step is a `.py` script or another
pipeline YAML):

```yaml
# pipeline.yaml
name: full

steps:
  - preprocess.py
  - train.py
```

Check and run:

```bash
riv check              # validate all pipelines
riv pipeline.yaml run  # execute (refuses to start unless it checks)
```

The checker verifies the wiring between steps: that artifacts are produced
before they are consumed, that producer and consumer agree on the schema, and
that every IO call is (eventually) typed. A mismatch renders like this:

```text
error[schema-mismatch]: `users.pkl` is consumed as `schemas.OrdersDf` but was produced as `schemas.UsersDf`
  --> train.py:3:10
   |
 3 | orders = OrdersDf.riv_in("users.pkl")
   |          ^^^^^^^^ consumed as `schemas.OrdersDf`
   |
  ::: preprocess.py:4:1
   |
 4 | UsersDf.riv_out(users, "users.pkl")
   | ------- produced as `schemas.UsersDf`
```

## Gradual typing

- Bare `riv_out(...)` is a **warning** by default: a partially migrated
  codebase still passes `riv check`.
- `strict: true` in the pipeline YAML promotes unannotated IO to an error.
- `Untyped.riv_out(blob, path)` is the explicit escape hatch — legal even
  under strict. `grep Untyped` is your tech-debt list.

## Documentation

- [Zero-cost adoption](https://github.com/licesma/riv/blob/main/documentation/zero-cost-adoption.md) —
  let riv be the glue between the scripts you already have: untyped first,
  typed whenever you feel like it.
- [Running pipelines](https://github.com/licesma/riv/blob/main/documentation/running-pipelines.md) —
  the pipeline YAML, `check` and `run`, and what execution actually does.

## Try the example

```bash
cd examples/hello
riv check
riv pipeline.yaml run
```
