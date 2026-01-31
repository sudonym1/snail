from __future__ import annotations

from ._native import __build_info__, compile, compile_ast, exec, parse, parse_ast


def _resolve_version() -> str:
    try:
        from importlib.metadata import version

        return version("snail-lang")
    except Exception:  # pragma: no cover - during development
        return "0.0.0"


def __getattr__(name: str):
    if name == "__version__":
        value = _resolve_version()
        globals()["__version__"] = value
        return value
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__() -> list[str]:
    return sorted(list(globals().keys()) + ["__version__"])


__all__ = [
    "compile",
    "compile_ast",
    "exec",
    "parse",
    "parse_ast",
    "__version__",
    "__build_info__",
]
