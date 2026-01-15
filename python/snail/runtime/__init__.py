from __future__ import annotations

import functools

from .compact_try import compact_try
from .regex import regex_compile, regex_search
from .structured_accessor import (
    __snail_jmespath_query,
    js,
)
from .subprocess import SubprocessCapture, SubprocessStatus

__all__ = ["install_helpers"]


def __snail_partial(func, /, *args, **kwargs):
    return functools.partial(func, *args, **kwargs)


def install_helpers(globals_dict: dict) -> None:
    globals_dict["__snail_compact_try"] = compact_try
    globals_dict["__snail_regex_search"] = regex_search
    globals_dict["__snail_regex_compile"] = regex_compile
    globals_dict["__SnailSubprocessCapture"] = SubprocessCapture
    globals_dict["__SnailSubprocessStatus"] = SubprocessStatus
    globals_dict["__snail_jmespath_query"] = __snail_jmespath_query
    globals_dict["__snail_partial"] = __snail_partial
    globals_dict["js"] = js
