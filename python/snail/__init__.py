from ._native import compile, exec, parse

try:
    from importlib.metadata import version

    __version__ = version("snail")
except Exception:  # pragma: no cover - during development
    __version__ = "0.0.0"

__all__ = ["compile", "exec", "parse", "__version__"]
