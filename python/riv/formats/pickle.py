"""Pickle format: .pkl / .pickle. Stdlib only."""

import pickle


def read(path):
    with open(path, "rb") as f:
        return pickle.load(f)


def write(obj, path):
    with open(path, "wb") as f:
        pickle.dump(obj, f)
