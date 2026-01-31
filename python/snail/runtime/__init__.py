from __future__ import annotations

import importlib
from typing import Optional

__all__ = ["install_helpers", "AutoImportDict", "AUTO_IMPORT_NAMES"]

# Names that can be auto-imported when first referenced.
# Maps name -> (module, attribute) where attribute is None for whole-module imports.
AUTO_IMPORT_NAMES: dict[str, tuple[str, Optional[str]]] = {
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

    def __missing__(self, key: str) -> object:
        if key in AUTO_IMPORT_NAMES:
            module_name, attr_name = AUTO_IMPORT_NAMES[key]
            module = importlib.import_module(module_name)
            value = getattr(module, attr_name) if attr_name else module
            self[key] = value
            return value
        raise KeyError(key)


_compact_try = None
_regex_search = None
_regex_compile = None
_subprocess_capture = None
_subprocess_status = None
_jmespath_query = None
_js = None
_lazy_text_class = None
_lazy_file_class = None
_incr_attr = None
_incr_index = None
_aug_attr = None
_aug_index = None


def _get_compact_try():
    global _compact_try
    if _compact_try is None:
        from .compact_try import compact_try

        _compact_try = compact_try
    return _compact_try


def _get_regex_search():
    global _regex_search
    if _regex_search is None:
        from .regex import regex_search

        _regex_search = regex_search
    return _regex_search


def _get_regex_compile():
    global _regex_compile
    if _regex_compile is None:
        from .regex import regex_compile

        _regex_compile = regex_compile
    return _regex_compile


def _get_subprocess_capture():
    global _subprocess_capture
    if _subprocess_capture is None:
        from .subprocess import SubprocessCapture

        _subprocess_capture = SubprocessCapture
    return _subprocess_capture


def _get_subprocess_status():
    global _subprocess_status
    if _subprocess_status is None:
        from .subprocess import SubprocessStatus

        _subprocess_status = SubprocessStatus
    return _subprocess_status


def _get_jmespath_query():
    global _jmespath_query
    if _jmespath_query is None:
        from .structured_accessor import __snail_jmespath_query

        _jmespath_query = __snail_jmespath_query
    return _jmespath_query


def _get_js():
    global _js
    if _js is None:
        from .structured_accessor import js

        _js = js
    return _js


def _get_lazy_text_class():
    global _lazy_text_class
    if _lazy_text_class is None:
        from .lazy_text import LazyText

        _lazy_text_class = LazyText
    return _lazy_text_class


def _get_lazy_file_class():
    global _lazy_file_class
    if _lazy_file_class is None:
        from .lazy_file import LazyFile

        _lazy_file_class = LazyFile
    return _lazy_file_class


def _get_incr_attr():
    global _incr_attr
    if _incr_attr is None:
        from .augmented import __snail_incr_attr

        _incr_attr = __snail_incr_attr
    return _incr_attr


def _get_incr_index():
    global _incr_index
    if _incr_index is None:
        from .augmented import __snail_incr_index

        _incr_index = __snail_incr_index
    return _incr_index


def _get_aug_attr():
    global _aug_attr
    if _aug_attr is None:
        from .augmented import __snail_aug_attr

        _aug_attr = __snail_aug_attr
    return _aug_attr


def _get_aug_index():
    global _aug_index
    if _aug_index is None:
        from .augmented import __snail_aug_index

        _aug_index = __snail_aug_index
    return _aug_index


def _lazy_compact_try(expr_fn, fallback_fn=None):
    return _get_compact_try()(expr_fn, fallback_fn)


def _lazy_regex_search(value, pattern):
    return _get_regex_search()(value, pattern)


def _lazy_regex_compile(pattern):
    return _get_regex_compile()(pattern)


def _lazy_subprocess_capture(cmd: str):
    return _get_subprocess_capture()(cmd)


def _lazy_subprocess_status(cmd: str):
    return _get_subprocess_status()(cmd)


def _lazy_jmespath_query(query: str):
    return _get_jmespath_query()(query)


def _lazy_js(input_data=None):
    return _get_js()(input_data)


def _lazy_incr_attr(obj, attr: str, delta: int, pre: bool):
    return _get_incr_attr()(obj, attr, delta, pre)


def _lazy_incr_index(obj, index, delta: int, pre: bool):
    return _get_incr_index()(obj, index, delta, pre)


def _lazy_aug_attr(obj, attr: str, value, op: str):
    return _get_aug_attr()(obj, attr, value, op)


def _lazy_aug_index(obj, index, value, op: str):
    return _get_aug_index()(obj, index, value, op)


def __snail_partial(func, /, *args, **kwargs):
    import functools

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
    globals_dict["__snail_compact_try"] = _lazy_compact_try
    globals_dict["__snail_regex_search"] = _lazy_regex_search
    globals_dict["__snail_regex_compile"] = _lazy_regex_compile
    globals_dict["__SnailSubprocessCapture"] = _lazy_subprocess_capture
    globals_dict["__SnailSubprocessStatus"] = _lazy_subprocess_status
    globals_dict["__snail_jmespath_query"] = _lazy_jmespath_query
    globals_dict["__snail_partial"] = __snail_partial
    globals_dict["__snail_contains__"] = __snail_contains__
    globals_dict["__snail_contains_not__"] = __snail_contains_not__
    globals_dict["__snail_incr_attr"] = _lazy_incr_attr
    globals_dict["__snail_incr_index"] = _lazy_incr_index
    globals_dict["__snail_aug_attr"] = _lazy_aug_attr
    globals_dict["__snail_aug_index"] = _lazy_aug_index
    globals_dict["js"] = _lazy_js
    globals_dict["__SnailLazyText"] = _get_lazy_text_class()
    globals_dict["__SnailLazyFile"] = _get_lazy_file_class()
