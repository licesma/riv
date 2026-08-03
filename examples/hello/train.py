from schemas import Metrics, UsersDf

users = UsersDf.riv_in("users.pkl")

metrics = {"n_users": len(users), "mean_name_len": sum(len(u["name"]) for u in users) / len(users)}

Metrics.riv_out(metrics, "metrics.json")
print(f"train: {metrics}")
