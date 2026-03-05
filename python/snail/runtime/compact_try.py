from __future__ import annotations


def compact_try_no_fallback(exc):
    fallback_member = getattr(exc, "__fallback__", None)
    if callable(fallback_member):
        return fallback_member()
    return None
