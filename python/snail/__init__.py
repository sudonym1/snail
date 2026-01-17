from ._native import __build_info__, compile, compile_ast, exec, parse

try:
    from importlib.metadata import version

    __version__ = version("snail-lang")
except Exception:  # pragma: no cover - during development
    __version__ = "0.0.0"

__all__ = ["compile", "compile_ast", "exec", "parse", "__version__", "__build_info__"]
