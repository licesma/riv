from riv import riv_in
from schemas import OrdersDf

orders = riv_in[OrdersDf]("users.pkl")
