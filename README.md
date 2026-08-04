# riv

Typed data pipelines for Python: a fast static checker (`riv check`) plus a
tiny, dependency-free runtime (`riv_in`, `riv_out`, `Schema`).

The model is TypeScript's: schemas are erased at runtime, checking is gradual,
and untyped riv is valid riv, forever.

```bash
pip install riv
```

## Get started

| | |
|---|---|
| [Your first river](https://github.com/licesma/riv/blob/main/documentation/your-first-river.md) | glue your Python scripts with a pipeline |
| [Running pipelines](https://github.com/licesma/riv/blob/main/documentation/running-pipelines.md) | `check`, run, and what execution does |

Or try the shipped example:

```bash
cd examples/hello
riv check
riv pipeline.yaml
```
