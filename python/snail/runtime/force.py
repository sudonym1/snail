"""Eager resolution of lazy proxy objects."""

from __future__ import annotations

from .lazy_proxy import LazyProxy


def force(value):
    """Resolve LazyProxy objects to their actual values (no-op for non-proxy)."""
    if isinstance(value, LazyProxy):
        return value._proxy_target()
    return value
