import pytest

import riv
from riv import Schema, UnknownFormatError, Untyped, register_format, riv_in, riv_out


class UsersDf(Schema): ...


def test_pickle_round_trip(tmp_path):
    users = [{"id": 1, "name": "ada"}, {"id": 2, "name": "grace"}]
    path = tmp_path / "users.pkl"
    riv_out(users, path)
    assert riv_in(path) == users


def test_pickle_extension_alias(tmp_path):
    path = tmp_path / "users.pickle"
    riv_out({"a": 1}, path)
    assert riv_in(path) == {"a": 1}


def test_json_round_trip(tmp_path):
    payload = {"metrics": [1.5, 2.5], "run": "full"}
    path = tmp_path / "metrics.json"
    riv_out(payload, path)
    assert riv_in(path) == payload


def test_schema_calls_are_erased_at_runtime(tmp_path):
    path = tmp_path / "users.pkl"
    UsersDf.riv_out([1, 2, 3], path)
    assert UsersDf.riv_in(path) == [1, 2, 3]
    # Any schema reads any artifact: schemas are contracts for `riv check`,
    # not runtime validators.
    assert Untyped.riv_in(path) == [1, 2, 3]


def test_unknown_extension_is_a_clear_error(tmp_path):
    with pytest.raises(UnknownFormatError, match=r"\.ply.*register_format"):
        riv_out(object(), tmp_path / "scene.ply")


def test_register_format_plugin(tmp_path):
    def read_txt(path):
        return open(path, encoding="utf-8").read()

    def write_txt(obj, path):
        open(path, "w", encoding="utf-8").write(obj)

    register_format(".txt", read_txt, write_txt)
    path = tmp_path / "note.txt"
    riv_out("hello", path)
    assert riv_in(path) == "hello"


def test_register_format_requires_leading_dot():
    with pytest.raises(ValueError, match="dot"):
        register_format("txt", lambda p: None, lambda o, p: None)


def test_csv_without_pandas_mentions_pandas(tmp_path, monkeypatch):
    import builtins

    real_import = builtins.__import__

    def no_pandas(name, *args, **kwargs):
        if name == "pandas":
            raise ImportError("No module named 'pandas'")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", no_pandas)
    with pytest.raises(ImportError, match="pandas is not installed"):
        riv_in(tmp_path / "users.csv")


def test_dataframe_formats_round_trip(tmp_path):
    pd = pytest.importorskip("pandas")
    df = pd.DataFrame({"id": [1, 2], "name": ["ada", "grace"]})
    csv_path = tmp_path / "users.csv"
    riv_out(df, csv_path)
    pd.testing.assert_frame_equal(riv_in(csv_path), df)


def test_version_is_exposed():
    assert riv.__version__
