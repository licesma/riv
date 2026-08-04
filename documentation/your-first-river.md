# Your first river

A river glues your Python scripts together.

Two scripts. One writes a file, the other reads it:

```python
# preprocess.py
import pandas as pd

df = pd.read_csv("users.csv")
df = df.dropna()
df.to_pickle("users.pkl")
```

```python
# train.py
import pandas as pd

df = pd.read_pickle("users.pkl")
model = train_model(df)
```

Write down the order in a YAML:

```yaml
# river.yaml
name: users

steps:
  - preprocess.py
  - train.py
```

Run it:

```bash
riv river.yaml
```

Each step runs in order, as a plain subprocess. Your scripts did not
change.

You got the idea, more scripts, more steps:

```yaml
name: users

steps:
  - preprocess.py
  - train.py
  - evaluate.py
```

That's your first river.

Next: [a typed river](typed-river.md).
