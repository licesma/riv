# A typed river

Same two scripts, with riv calls instead of pickle calls:

```python
# preprocess.py
from riv import riv_out
import pandas as pd

df = pd.read_csv("users.csv")
df = df.dropna()
riv_out(df, "users.pkl")
```

```python
# train.py
from riv import riv_in

df = riv_in("users.pkl")
model = train_model(df)
```

`riv_out` and `riv_in` do exactly what the pickle calls did. Now riv
sees the data flow: `riv check` verifies every artifact is produced
before it is consumed.

Then add types. A schema is a name for what flows:

```python
# schemas.py
from riv import Schema

class UsersDf(Schema): ...
```

```python
# preprocess.py
from schemas import UsersDf

riv_out[UsersDf](df, "users.pkl")
```

```python
# train.py
from schemas import UsersDf

df = riv_in[UsersDf]("users.pkl")
```

Both sides now name the contract. Read the wrong schema and `riv check`
shows both sides:

```text
error[schema-mismatch]: `users.pkl` is consumed as `schemas.OrdersDf` but was produced as `schemas.UsersDf`
  --> train.py:4:17
   |
 4 | orders = riv_in[OrdersDf]("users.pkl")
   |                 ^^^^^^^^ consumed as `schemas.OrdersDf`
   |
  ::: preprocess.py:5:9
   |
 5 | riv_out[UsersDf](users, "users.pkl")
   |         ------- produced as `schemas.UsersDf`
```

Schemas are erased at runtime; storage does not change.

Type one edge at a time. Untyped calls warn, they never fail the check.
The explicit opt-out is `riv_out[Untyped]`, legal even under
`strict: true`, which turns the remaining warnings into errors.
