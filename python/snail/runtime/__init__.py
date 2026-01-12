from __future__ import annotations

from .compact_try import compact_try
from .regex import regex_compile, regex_search
from .structured_accessor import (
    JsonObject,
    JsonPipelineWrapper,
    StructuredAccessor,
    json,
)
from .subprocess import SubprocessCapture, SubprocessStatus

__all__ = ["install_helpers"]


def install_helpers(globals_dict: dict) -> None:
    globals_dict["__snail_compact_try"] = compact_try
    globals_dict["__snail_regex_search"] = regex_search
    globals_dict["__snail_regex_compile"] = regex_compile
    globals_dict["__SnailSubprocessCapture"] = SubprocessCapture
    globals_dict["__SnailSubprocessStatus"] = SubprocessStatus
    globals_dict["__SnailStructuredAccessor"] = StructuredAccessor
    globals_dict["__SnailJsonObject"] = JsonObject
    globals_dict["__SnailJsonPipelineWrapper"] = JsonPipelineWrapper
    globals_dict["json"] = json
