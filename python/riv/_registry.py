"""Extension -> format-plugin dispatch.

Built-in formats are referenced by module name and imported lazily, so
that heavyweight third-party libraries (pandas, pyarrow) are only touched
when an artifact of that format is actually read or written.
"""

import importlib
import os

_BUILTIN_FORMATS = {
    ".pkl": "riv.formats.pickle",
    ".pickle": "riv.formats.pickle",
    ".json": "riv.formats.json",
    ".csv": "riv.formats.csv",
    ".parquet": "riv.formats.parquet",
}

# extension -> format object (anything with read(path) / write(obj, path))
_registered = {}


class UnknownFormatError(Exception):
    """Raised when no format plugin is registered for a file extension."""


class _CallableFormat:
    def __init__(self, reader, writer):
        self.read = reader
        self.write = writer


def register_format(extension, reader, writer):
    """Register a format plugin for a file extension.

    ``reader(path) -> obj`` and ``writer(obj, path) -> None``. The
    extension must include the leading dot, e.g. ``".ply"``. Overrides
    built-ins and earlier registrations.
    """
    if not extension.startswith("."):
        raise ValueError(f"extension must start with a dot, got {extension!r}")
    _registered[extension.lower()] = _CallableFormat(reader, writer)


def resolve_format(path):
    """Return the format plugin for `path`, keyed by its extension."""
    extension = os.path.splitext(os.fspath(path))[1].lower()
    if extension in _registered:
        return _registered[extension]
    module_name = _BUILTIN_FORMATS.get(extension)
    if module_name is None:
        known = sorted(set(_BUILTIN_FORMATS) | set(_registered))
        raise UnknownFormatError(
            f"no riv format is registered for {extension!r} "
            f"(from {os.fspath(path)!r}); known extensions: {', '.join(known)}. "
            "Use riv.register_format() to add one."
        )
    return importlib.import_module(module_name)
