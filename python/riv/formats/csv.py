"""CSV format: .csv. Requires pandas (the user's, not riv's)."""


def _pandas():
    try:
        import pandas
    except ImportError as e:
        raise ImportError(
            "riv's .csv format reads and writes pandas DataFrames, but pandas "
            "is not installed in this environment. Install pandas to use it; "
            "riv itself has no dependencies."
        ) from e
    return pandas


def read(path):
    return _pandas().read_csv(path)


def write(obj, path):
    obj.to_csv(path, index=False)
