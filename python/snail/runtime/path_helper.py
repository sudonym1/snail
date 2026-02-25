from __future__ import annotations

import glob
import os
from pathlib import Path


class GlobError(Exception):
    """Raised when a glob pattern matches no files."""

    __fallback__: object


def path(*patterns: str | os.PathLike[str]) -> list[Path]:
    """Expand glob patterns and return existing matching paths.

    Raises GlobError if any pattern matches nothing. The fallback
    contains the partial results from patterns that did match.
    """
    results: list[Path] = []
    failed: list[str] = []
    for pattern in patterns:
        pat = os.fspath(pattern) if isinstance(pattern, os.PathLike) else pattern
        matches = glob.glob(pat)
        if matches:
            results.extend(Path(p) for p in matches)
        else:
            failed.append(pat)
    if failed:
        partial = list(results)
        err = GlobError(f"no matches: {' '.join(failed)}")
        err.__fallback__ = lambda: partial
        raise err
    return results
