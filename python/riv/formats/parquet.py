"""Parquet format: .parquet. Requires pandas + a parquet engine (user's deps)."""


def _pandas():
    try:
        import pandas
    except ImportError as e:
        raise ImportError(
            "riv's .parquet format reads and writes pandas DataFrames, but "
            "pandas is not installed in this environment. Install pandas and "
            "a parquet engine (pyarrow or fastparquet) to use it; riv itself "
            "has no dependencies."
        ) from e
    return pandas


def read(path):
    return _pandas().read_parquet(path)


def write(obj, path):
    obj.to_parquet(path)
