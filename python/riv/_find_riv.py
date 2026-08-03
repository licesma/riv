"""Locate the `riv` binary that maturin placed in the scripts directory.

Modeled on ruff's `_find_ruff.py`: the wheel carries the native binary in
`.data/scripts/`, so pip installs it next to the interpreter's other
scripts. `python -m riv` resolves it from here and exec's it.
"""

import os
import sys
import sysconfig


def find_riv_bin() -> str:
    """Return the path to the riv binary, or raise FileNotFoundError."""
    riv_exe = "riv" + sysconfig.get_config_var("EXE")

    scripts_path = os.path.join(sysconfig.get_path("scripts"), riv_exe)
    if os.path.isfile(scripts_path):
        return scripts_path

    if sys.version_info >= (3, 10):
        user_scheme = sysconfig.get_preferred_scheme("user")
    elif os.name == "nt":
        user_scheme = "nt_user"
    elif sys.platform == "darwin" and sys._framework:
        user_scheme = "osx_framework_user"
    else:
        user_scheme = "posix_user"

    user_path = os.path.join(sysconfig.get_path("scripts", scheme=user_scheme), riv_exe)
    if os.path.isfile(user_path):
        return user_path

    # `pip install --prefix` and `--target` layouts: the scripts directory
    # sits next to this package's grandparent.
    pkg_root = os.path.dirname(os.path.dirname(__file__))
    target_path = os.path.join(pkg_root, "bin", riv_exe)
    if os.path.isfile(target_path):
        return target_path

    raise FileNotFoundError(scripts_path)
