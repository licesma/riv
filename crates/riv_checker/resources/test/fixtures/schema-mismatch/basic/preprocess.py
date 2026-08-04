from riv import riv_out
from schemas import UsersDf

users = [1, 2, 3]
riv_out[UsersDf](users, "users.pkl")
