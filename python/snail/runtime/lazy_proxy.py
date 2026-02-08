"""Shared lazy proxy helpers for map-mode runtime objects."""

from __future__ import annotations


class LazyProxy:
    """Base class that forwards attribute and iteration access lazily."""

    __slots__ = ()

    def _proxy_target(self):
        raise NotImplementedError

    def __getattr__(self, name):
        return getattr(self._proxy_target(), name)

    def __iter__(self):
        return iter(self._proxy_target())
