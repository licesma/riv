# riv

Typed data pipelines for Python: a fast static checker (`riv check`) plus a
tiny, dependency-free runtime (`riv_in`, `riv_out`, `Schema`).

The model is TypeScript's: schemas are erased at runtime, checking is gradual,
and untyped riv is valid riv, forever.

```bash
pip install riv
```

## Sneak peek

A pipeline (a **river**) is one YAML file. Steps run in the order listed; a
step is a Python script or another pipeline:

```yaml
# pipeline.yaml
name: full

steps:
  - preprocess.yaml
  - train.py
```

```yaml
# preprocess.yaml
name: preprocess

steps:
  - download.py
  - strip.py
```

```bash
riv check                # validate every pipeline under the current directory
riv pipeline.yaml check  # validate one pipeline
riv pipeline.yaml        # execute one pipeline
```

## Get started

| | |
|---|---|
| [Your first river](https://github.com/licesma/riv/blob/main/documentation/your-first-river.md) | glue your Python scripts with a pipeline |
| [A typed river](https://github.com/licesma/riv/blob/main/documentation/typed-river.md) | schemas make the wiring checkable |
