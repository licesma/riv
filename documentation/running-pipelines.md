# Running pipelines

A pipeline is one YAML file. Steps run in the order listed; a step is
either a Python script or another pipeline YAML:

```yaml
# pipeline.yaml
name: full

steps:
  - preprocess.py
  - train.py
```

```yaml
# all.yaml — pipelines compose recursively
name: all

steps:
  - full.yaml
  - benchmark.yaml
```

## Commands

```bash
riv check              # validate every pipeline under the current directory
riv pipeline.yaml check  # validate one pipeline
riv pipeline.yaml run    # execute one pipeline
```

`run` implies `check`: a pipeline that doesn't type-check refuses to
start, so a schema mismatch is caught before step 1 runs instead of
crashing step 4. The escape hatch is `riv pipeline.yaml run --no-check`.

## What execution means

Each `.py` step runs as a plain subprocess, one after another; a step
that exits non-zero stops the pipeline and `riv run` propagates its exit
code. A `.yaml` step runs that pipeline's steps in place.

Steps run in your ambient environment — riv does no environment
management:

- **Interpreter**: `$RIV_PYTHON` if set, else `python3`, else `python`.
- **Environment variables**: inherited from your shell.
- **Working directory**: the directory of the pipeline YAML that lists
  the step, so relative artifact paths like `users.pkl` resolve next to
  the YAML that wired them.

The only requirement is that `riv` is importable by the interpreter your
steps run under (it's `pip install riv` — the runtime has zero
dependencies).

## Try it

The repository ships a working example:

```bash
cd examples/hello
riv check
riv pipeline.yaml run
```

New to riv? Start with [zero-cost adoption](zero-cost-adoption.md).
