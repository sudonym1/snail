from __future__ import annotations

import os
from pathlib import Path

_GLOB_CHARS = frozenset("*?[")


class GlobError(Exception):
    """Raised when a glob pattern matches no files."""

    __fallback__: object


def _has_glob_chars(pattern: str) -> bool:
    return any(c in _GLOB_CHARS for c in pattern)


def _expand_pattern(pat: str) -> list[Path]:
    if not _has_glob_chars(pat):
        p = Path(pat)
        return [p] if p.exists() else []

    p = Path(pat)
    if p.is_absolute():
        parts = p.parts
        glob_start = next(i for i, part in enumerate(parts) if _has_glob_chars(part))
        base = Path(*parts[:glob_start])
        relative = str(Path(*parts[glob_start:]))
        return list(base.glob(relative))
    else:
        return list(Path(".").glob(pat))


def path(*patterns: str | os.PathLike[str]) -> list[Path]:
    """Expand glob patterns and return existing matching paths.

    Raises GlobError if any pattern matches nothing. The fallback
    contains the partial results from patterns that did match.
    """
    results: list[Path] = []
    failed: list[str] = []
    for pattern in patterns:
        pat = os.fspath(pattern) if isinstance(pattern, os.PathLike) else pattern
        matches = _expand_pattern(pat)
        if matches:
            results.extend(matches)
        else:
            failed.append(pat)
    if failed:
        partial = list(results)
        err = GlobError(f"no matches: {' '.join(failed)}")
        err.__fallback__ = lambda: partial
        raise err
    return results
