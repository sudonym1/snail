from __future__ import annotations


def compact_try(expr_fn, fallback_fn=None):
    try:
        return expr_fn()
    except Exception as exc:
        if fallback_fn is None:
            fallback_member = getattr(exc, "__fallback__", None)
            if callable(fallback_member):
                return fallback_member()
            return None
        return fallback_fn(exc)
