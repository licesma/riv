from riv import riv_in, riv_out
from schemas import Metrics, UsersDf

users = riv_in[UsersDf]("users.pkl")

metrics = {"n_users": len(users), "mean_name_len": sum(len(u["name"]) for u in users) / len(users)}

riv_out[Metrics](metrics, "metrics.json")
print(f"train: {metrics}")
