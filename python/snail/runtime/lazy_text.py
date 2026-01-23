"""Lazy text content reader for map mode."""

from __future__ import annotations


class LazyText:
    """Lazily reads file content on first access."""

    __slots__ = ("_fd", "_text")

    def __init__(self, fd):
        self._fd = fd
        self._text = None

    def _ensure_loaded(self):
        if self._text is None:
            self._text = self._fd.read()
        return self._text

    def __str__(self):
        return self._ensure_loaded()

    def __repr__(self):
        return repr(str(self))

    def __eq__(self, other):
        if isinstance(other, LazyText):
            return str(self) == str(other)
        return str(self) == other

    def __hash__(self):
        return hash(str(self))

    def __len__(self):
        return len(str(self))

    def __iter__(self):
        return iter(str(self))

    def __contains__(self, item):
        return item in str(self)

    def __add__(self, other):
        return str(self) + other

    def __radd__(self, other):
        return other + str(self)

    def __getattr__(self, name):
        return getattr(str(self), name)
