from schemas import UsersDf

users = [
    {"id": 1, "name": "ada"},
    {"id": 2, "name": "grace"},
    {"id": 3, "name": "margaret"},
]

UsersDf.riv_out(users, "users.pkl")
print(f"preprocess: wrote {len(users)} users")
