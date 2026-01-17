from __future__ import annotations

import functools
import importlib
from typing import Any

from .compact_try import compact_try
from .regex import regex_compile, regex_search
from .structured_accessor import (
    __snail_jmespath_query,
    js,
)
from .subprocess import SubprocessCapture, SubprocessStatus

__all__ = ["install_helpers", "AutoImportDict", "AUTO_IMPORT_NAMES"]

# Names that can be auto-imported when first referenced.
# Maps name -> (module, attribute) where attribute is None for whole-module imports.
AUTO_IMPORT_NAMES: dict[str, tuple[str, str | None]] = {
    # Whole module imports: import X
    "sys": ("sys", None),
    "os": ("os", None),
    # Attribute imports: from X import Y
    "Path": ("pathlib", "Path"),
}


class AutoImportDict(dict):
    """A dict subclass that lazily imports allowed names on first access.

    When a key lookup fails, if the key is in AUTO_IMPORT_NAMES,
    the corresponding module/attribute is imported and stored in the dict.
    Supports both whole-module imports (import sys) and attribute imports
    (from pathlib import Path).
    """

    def __missing__(self, key: str) -> Any:
        if key in AUTO_IMPORT_NAMES:
            module_name, attr_name = AUTO_IMPORT_NAMES[key]
            module = importlib.import_module(module_name)
            value = getattr(module, attr_name) if attr_name else module
            self[key] = value
            return value
        raise KeyError(key)


def __snail_partial(func, /, *args, **kwargs):
    return functools.partial(func, *args, **kwargs)


def __snail_contains__(left, right):
    method = getattr(right, "__snail_contains__", None)
    if method is not None:
        return method(left)
    return left in right


def __snail_contains_not__(left, right):
    method = getattr(right, "__snail_contains__", None)
    if method is not None:
        return not bool(method(left))
    return left not in right


def install_helpers(globals_dict: dict) -> None:
    globals_dict["__snail_compact_try"] = compact_try
    globals_dict["__snail_regex_search"] = regex_search
    globals_dict["__snail_regex_compile"] = regex_compile
    globals_dict["__SnailSubprocessCapture"] = SubprocessCapture
    globals_dict["__SnailSubprocessStatus"] = SubprocessStatus
    globals_dict["__snail_jmespath_query"] = __snail_jmespath_query
    globals_dict["__snail_partial"] = __snail_partial
    globals_dict["__snail_contains__"] = __snail_contains__
    globals_dict["__snail_contains_not__"] = __snail_contains_not__
    globals_dict["js"] = js
