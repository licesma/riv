"""riv runtime: typed artifact IO for rivers (data pipelines).

The runtime is deliberately tiny and pure Python. Schemas are erased at
runtime (like TypeScript annotations): ``riv_in[UsersDf](path)`` behaves
exactly like ``riv_in(path)``. The schema exists for `riv check`, the
static checker that ships as the `riv` binary in the same wheel.
"""

from typing import Any, Callable, Type, TypeVar

from riv._registry import UnknownFormatError, register_format, resolve_format

__version__ = "0.1.0"

__all__ = [
    "Schema",
    "UnknownFormatError",
    "Untyped",
    "register_format",
    "riv_in",
    "riv_out",
]

S = TypeVar("S", bound="Schema")


class _RivIn:
    """``riv_in``: bring an artifact into this script.

    Call it bare (``riv_in(path)``) or annotate the edge with a schema
    (``riv_in[UsersDf](path)``). The subscript is erased: both forms read
    identically, dispatching on file extension. The schema exists for
    `riv check` and for Python type checkers, which see ``UsersDf`` as
    the type of the returned value.
    """

    def __call__(self, path: Any) -> Any:
        return resolve_format(path).read(path)

    def __getitem__(self, schema: "Type[S]") -> "Callable[[Any], S]":
        return self  # type: ignore[return-value]


class _RivOut:
    """``riv_out``: send an artifact out of this script.

    Call it bare (``riv_out(obj, path)``) or annotate the edge with a
    schema (``riv_out[UsersDf](df, path)``). The subscript is erased:
    both forms write identically, dispatching on file extension.
    """

    def __call__(self, obj: Any, path: Any) -> None:
        resolve_format(path).write(obj, path)

    def __getitem__(self, schema: "Type[S]") -> "Callable[[S, Any], None]":
        return self  # type: ignore[return-value]


riv_in = _RivIn()
riv_out = _RivOut()


class Schema:
    """Base class for artifact schemas.

    Subclass it to name a contract, then attach it to IO calls::

        class UsersDf(Schema): ...

        riv_out[UsersDf](df, "users.pkl")
        df = riv_in[UsersDf]("users.pkl")

    At runtime the schema is erased; `riv check` verifies that producers
    and consumers of the same artifact agree on the schema symbol.
    """


class Untyped(Schema):
    """The explicit escape hatch (riv's `any`).

    ``riv_out[Untyped](blob, path)`` is a decision, not an accident: it
    is legal even under ``strict: true`` and emits no diagnostic, unlike
    a bare ``riv_out(blob, path)``. `grep Untyped` is the tech-debt list.
    """
