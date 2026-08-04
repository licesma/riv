from riv import riv_in
from schemas import Model, UsersDf

users = riv_in[UsersDf]("users.pkl")
model = riv_in[Model]("model.pkl")
