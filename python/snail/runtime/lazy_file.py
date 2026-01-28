"""Lazy file opener for map mode."""

from __future__ import annotations


class LazyFile:
    """Context manager that opens the file on first access."""

    __slots__ = ("_path", "_mode", "_kwargs", "_fd", "_closed")

    def __init__(self, path, mode="r", **kwargs):
        self._path = path
        self._mode = mode
        self._kwargs = kwargs
        self._fd = None
        self._closed = False

    def _ensure_open(self):
        if self._closed:
            raise ValueError("I/O operation on closed file.")
        if self._fd is None:
            self._fd = open(self._path, self._mode, **self._kwargs)
        return self._fd

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        self._closed = True
        if self._fd is not None:
            self._fd.close()
        return False

    def __getattr__(self, name):
        return getattr(self._ensure_open(), name)

    def __iter__(self):
        return iter(self._ensure_open())

    def __next__(self):
        return next(self._ensure_open())
