"""`python -m riv`: find the bundled binary and hand over to it."""

import os
import sys

from riv._find_riv import find_riv_bin

if __name__ == "__main__":
    riv = os.fsdecode(find_riv_bin())
    if sys.platform == "win32":
        import subprocess

        completed_process = subprocess.run([riv, *sys.argv[1:]], check=False)
        sys.exit(completed_process.returncode)
    else:
        os.execvp(riv, [riv, *sys.argv[1:]])
