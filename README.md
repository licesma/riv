# riv

Typed data rivers for Python: a fast static checker (`riv check`) plus a
tiny, dependency-free runtime (`riv_in`, `riv_out`, `Schema`).

A river is a data pipeline: your scripts, glued and checked. The model is
TypeScript's: schemas are erased at runtime, checking is gradual, and
untyped riv is valid riv, forever.

```bash
pip install riv
```

## Sneak peek

A river is one YAML file. Steps run in the order listed; a step is a
Python script or another river:

```yaml
# river.yaml
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
riv check              # validate every river under the current directory
riv river.yaml check   # validate one river
riv river.yaml         # execute one river
riv train              # execute train/river.yaml
```

## Get started

| | |
|---|---|
| [Your first river](https://github.com/licesma/riv/blob/main/documentation/your-first-river.md) | glue your Python scripts with a river |
| [A typed river](https://github.com/licesma/riv/blob/main/documentation/typed-river.md) | schemas make the wiring checkable |
