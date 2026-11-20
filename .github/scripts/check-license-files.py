"""Check that Python wheels and sdists carry richdem-rs's license files.

Usage: python .github/scripts/check-license-files.py python/dist/*.whl python/dist/*.tar.gz

A wheel must hold LICENSE and NOTICE in its ``.dist-info/licenses/`` directory, and an sdist
must hold them in both ``python/`` (where maturin finds them when building from source) and at
the archive root (where PyPI looks for the ``License-File`` metadata entries).
"""

from __future__ import annotations

import sys
import tarfile
import zipfile

NAMES = ("LICENSE", "NOTICE")


def members(path: str) -> list[str]:
    """List member paths inside a wheel (`.whl`) or source distribution (`.tar.gz`)."""
    if path.endswith(".whl"):
        with zipfile.ZipFile(path) as wheel:
            return wheel.namelist()
    with tarfile.open(path) as sdist:
        return sdist.getnames()


def main() -> None:
    """Verify every archive on the command line carries all required license files."""
    missing: list[str] = []
    for path in sys.argv[1:]:
        names = members(path)
        for name in NAMES:
            if path.endswith(".whl"):
                found = any(n.endswith(f".dist-info/licenses/{name}") for n in names)
                if not found:
                    missing.append(f"{path}: {name}")
                continue
            inside = {n.split("/", 1)[-1] for n in names}
            for place in (f"python/{name}", name):
                if place not in inside:
                    missing.append(f"{path}: {place}")
    if not sys.argv[1:]:
        missing.append("no wheel or sdist given")
    for line in missing:
        print(f"missing {line}", file=sys.stderr)
    sys.exit(1 if missing else 0)


if __name__ == "__main__":
    main()
