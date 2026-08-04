# Zero-cost adoption

Let riv be the glue between the Python scripts you already have. No
rewrite, no framework, no decorators, no base classes to inherit from —
your scripts stay plain Python, and riv only cares about one thing: how
data moves between them.

Say you have a couple of scripts that pass data through files:

```python
# preprocess.py
import pandas as pd

df = load_and_clean_the_usual_way()
df.to_pickle("users.pkl")
```

```python
# train.py
import pandas as pd

df = pd.read_pickle("users.pkl")
train_on(df)
```

That already works, of course. What it doesn't have is a contract:
nothing tells you `train.py` must run after `preprocess.py`, and nothing
catches it when someone renames the file, changes what's inside it, or
reorders the scripts.

## Step 1 — untyped

Swap the pickle round-trips for riv's untyped form:

```python
# preprocess.py
from riv import riv_out

df = load_and_clean_the_usual_way()
riv_out(df, "users.pkl")
```

```python
# train.py
from riv import riv_in

df = riv_in("users.pkl")
train_on(df)
```

and write down the order in a small YAML:

```yaml
# pipeline.yaml
name: demo

steps:
  - preprocess.py
  - train.py
```

That's the whole migration. Everything works right now — `riv_out` /
`riv_in` do exactly what your pickle calls did. `riv check` will warn
that the IO is untyped, but a warning is not an error: **untyped riv is
valid riv, forever**. You can already [run the
pipeline](running-pipelines.md).

What you get for those five minutes: `riv check` verifies that every
artifact is produced before it's consumed. Reorder the steps, rename an
artifact on one side only, or consume a file nobody writes, and the
checker tells you — before anything runs.

## Step 2 — typed, whenever you feel like it

Attach a schema to the edge. A schema is just a class:

```python
# schemas.py
from riv import Schema

class UsersDf(Schema): ...
```

```python
# preprocess.py
from schemas import UsersDf

df = load_and_clean_the_usual_way()
UsersDf.riv_out(df, "users.pkl")
```

```python
# train.py
from schemas import UsersDf

df = UsersDf.riv_in("users.pkl")
train_on(df)
```

Now producer and consumer point at the same name, and `riv check`
verifies they agree — if `train.py` reads the file as `OrdersDf`, that's
an error with both locations in the message. The schema is erased at
runtime: nothing about how your data is stored or loaded changes.

Migrate one edge at a time. Typed and untyped calls coexist in the same
pipeline, the same script, even the same line count you had before —
that's the zero-cost part.

## Already typed?

Then there is no step 3 — you're done. Go [run your
pipelines](running-pipelines.md).
