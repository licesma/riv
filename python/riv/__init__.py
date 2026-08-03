"""riv runtime: typed artifact IO for data pipelines.

The runtime is deliberately tiny and pure Python. Schemas are erased at
runtime (like TypeScript annotations): ``UsersDf.riv_in(path)`` behaves
exactly like ``riv_in(path)``. The schema exists for `riv check`, the
static checker that ships as the `riv` binary in the same wheel.
"""

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


def riv_in(path):
    """Bring an artifact into this script. Dispatches on file extension."""
    return resolve_format(path).read(path)


def riv_out(obj, path):
    """Send an artifact out of this script. Dispatches on file extension."""
    resolve_format(path).write(obj, path)


class Schema:
    """Base class for artifact schemas.

    Subclass it to name a contract, then attach it to IO calls::

        class UsersDf(Schema): ...

        UsersDf.riv_out(df, "users.pkl")
        df = UsersDf.riv_in("users.pkl")

    At runtime the schema is erased; `riv check` verifies that producers
    and consumers of the same artifact agree on the schema symbol.
    """

    @classmethod
    def riv_in(cls, path):
        return riv_in(path)

    @classmethod
    def riv_out(cls, obj, path):
        riv_out(obj, path)


class Untyped(Schema):
    """The explicit escape hatch (riv's `any`).

    ``Untyped.riv_out(blob, path)`` is a decision, not an accident: it is
    legal even under ``strict: true`` and emits no diagnostic, unlike a
    bare ``riv_out(blob, path)``. `grep Untyped` is the tech-debt list.
    """
