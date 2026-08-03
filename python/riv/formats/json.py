"""JSON format: .json. Stdlib only."""

import json


def read(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def write(obj, path):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(obj, f)
