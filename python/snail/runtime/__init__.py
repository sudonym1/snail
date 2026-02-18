from __future__ import annotations

import contextlib
import importlib
import re
import sys
from typing import Any, Callable, Optional

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


_GETTER_CACHE: dict[str, Any] = {}
_GETTERS: dict[str, Callable[[], Any]] = {}
_LAZY_WRAPPERS: dict[str, Callable[..., Any]] = {}
_awk_split_cache: dict[tuple[str, bool], re.Pattern[str]] = {}

_GETTER_REGISTRY: dict[str, tuple[str, str, bool]] = {
    "_get_compact_try": (".compact_try", "compact_try", False),
    "_get_regex_search": (".regex", "regex_search", False),
    "_get_regex_compile": (".regex", "regex_compile", False),
    "_get_subprocess_capture": (".subprocess", "SubprocessCapture", False),
    "_get_subprocess_status": (".subprocess", "SubprocessStatus", False),
    "_get_jmespath_query": (".structured_accessor", "__snail_jmespath_query", False),
    "_get_js": (".structured_accessor", "js", False),
    "_get_lazy_text_class": (".lazy_text", "LazyText", False),
    "_get_lazy_file_class": (".lazy_file", "LazyFile", False),
    "_get_env_map": (".env", "EnvMap", True),
    "_get_incr_attr": (".augmented", "__snail_incr_attr", False),
    "_get_incr_index": (".augmented", "__snail_incr_index", False),
    "_get_aug_attr": (".augmented", "__snail_aug_attr", False),
    "_get_aug_index": (".augmented", "__snail_aug_index", False),
    "_get_path": (".path_helper", "path", False),
}

_LAZY_WRAPPER_REGISTRY: dict[str, str] = {
    "_lazy_compact_try": "_get_compact_try",
    "_lazy_regex_search": "_get_regex_search",
    "_lazy_regex_compile": "_get_regex_compile",
    "_lazy_subprocess_capture": "_get_subprocess_capture",
    "_lazy_subprocess_status": "_get_subprocess_status",
    "_lazy_jmespath_query": "_get_jmespath_query",
    "_lazy_js": "_get_js",
    "_lazy_incr_attr": "_get_incr_attr",
    "_lazy_incr_index": "_get_incr_index",
    "_lazy_aug_attr": "_get_aug_attr",
    "_lazy_aug_index": "_get_aug_index",
    "_lazy_path": "_get_path",
}

_INSTALL_LAZY_HELPER_REGISTRY: dict[str, str] = {
    "__snail_compact_try": "_lazy_compact_try",
    "__snail_regex_search": "_lazy_regex_search",
    "__snail_regex_compile": "_lazy_regex_compile",
    "__SnailSubprocessCapture": "_lazy_subprocess_capture",
    "__SnailSubprocessStatus": "_lazy_subprocess_status",
    "__snail_jmespath_query": "_lazy_jmespath_query",
    "__snail_incr_attr": "_lazy_incr_attr",
    "__snail_incr_index": "_lazy_incr_index",
    "__snail_aug_attr": "_lazy_aug_attr",
    "__snail_aug_index": "_lazy_aug_index",
    "js": "_lazy_js",
    "path": "_lazy_path",
}

_INSTALL_EAGER_HELPER_REGISTRY: dict[str, str] = {
    "__snail_env": "_get_env_map",
    "__SnailLazyText": "_get_lazy_text_class",
    "__SnailLazyFile": "_get_lazy_file_class",
}


def _load_helper(module_name: str, attr_name: str) -> Any:
    module = importlib.import_module(module_name, package=__name__)
    return getattr(module, attr_name)


def _make_cached_getter(
    getter_name: str, module_name: str, attr_name: str, *, instantiate: bool
) -> Callable[[], Any]:
    def getter() -> Any:
        value = _GETTER_CACHE.get(getter_name)
        if value is None:
            loaded = _load_helper(module_name, attr_name)
            value = loaded() if instantiate else loaded
            _GETTER_CACHE[getter_name] = value
        return value

    getter.__name__ = getter_name
    getter.__qualname__ = getter_name
    return getter


def _make_lazy_wrapper(wrapper_name: str, getter_name: str) -> Callable[..., Any]:
    getter = _GETTERS[getter_name]

    def wrapper(*args: Any, **kwargs: Any) -> Any:
        return getter()(*args, **kwargs)

    wrapper.__name__ = wrapper_name
    wrapper.__qualname__ = wrapper_name
    return wrapper


for _getter_name, (_module_name, _attr_name, _instantiate) in _GETTER_REGISTRY.items():
    _getter = _make_cached_getter(
        _getter_name, _module_name, _attr_name, instantiate=_instantiate
    )
    _GETTERS[_getter_name] = _getter
    globals()[_getter_name] = _getter


for _wrapper_name, _getter_name in _LAZY_WRAPPER_REGISTRY.items():
    _wrapper = _make_lazy_wrapper(_wrapper_name, _getter_name)
    _LAZY_WRAPPERS[_wrapper_name] = _wrapper
    globals()[_wrapper_name] = _wrapper


def __snail_awk_split_internal(
    line: str, separators: Optional[str], include_whitespace: bool
):
    if not separators:
        return line.split()
    if not include_whitespace:
        if len(separators) == 1:
            return line.split(separators)
        regex = _awk_split_cache.get((separators, False))
        if regex is None:
            regex = re.compile(f"[{re.escape(separators)}]")
            _awk_split_cache[(separators, False)] = regex
        return regex.split(line)

    stripped = line.strip()
    if not stripped:
        return []
    regex = _awk_split_cache.get((separators, True))
    if regex is None:
        regex = re.compile(f"(?:\\s+|[{re.escape(separators)}])")
        _awk_split_cache[(separators, True)] = regex
    return regex.split(stripped)


def __snail_awk_split(line: str, separators: Optional[str], include_whitespace: bool):
    fields = __snail_awk_split_internal(line, separators, include_whitespace)
    return [f for f in fields if f]


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


def __snail_lines_iter(source):
    """Iterate lines from a source for 'lines(expr) { }' blocks.

    Handles:
    - str: opens as file path, iterates lines
    - file-like (has readline): iterates lines directly
    - other iterable: iterates directly
    """
    if isinstance(source, str):
        with open(source) as f:
            yield from f
    elif hasattr(source, "readline"):
        yield from source
    else:
        yield from source


@contextlib.contextmanager
def __snail_open_lines_source(source):
    """Context manager: open a lines() source item for reading.

    - str: open as file path (or stdin for "-")
    - file-like (has readline): use directly, don't close
    - other: TypeError
    """
    if isinstance(source, str):
        if source == "-":
            yield sys.stdin, "-"
        else:
            with open(source) as f:
                yield f, source
    elif hasattr(source, "readline"):
        yield source, getattr(source, "name", "<source>")
    else:
        raise TypeError(
            f"lines() source must be a file path or file-like object, got {type(source).__name__}"
        )


def __snail_normalize_sources(*sources):
    """Normalize lines()/files() source arguments to a flat list of sources.

    Single arg:
    - str: [source]  (single file path)
    - file-like: [source]  (single file object)
    - other iterable: list(source)  (list of sources)

    Multiple args: each arg is treated as an individual source.
    """
    if len(sources) != 1:
        return list(sources)
    source = sources[0]
    if isinstance(source, str) or hasattr(source, "readline"):
        return [source]
    return list(source)


def install_helpers(globals_dict: dict) -> None:
    for helper_name, wrapper_name in _INSTALL_LAZY_HELPER_REGISTRY.items():
        globals_dict[helper_name] = _LAZY_WRAPPERS[wrapper_name]

    globals_dict["__snail_partial"] = __snail_partial
    globals_dict["__snail_contains__"] = __snail_contains__
    globals_dict["__snail_contains_not__"] = __snail_contains_not__
    globals_dict["__snail_awk_split"] = __snail_awk_split
    globals_dict["__snail_lines_iter"] = __snail_lines_iter
    globals_dict["__snail_open_lines_source"] = __snail_open_lines_source
    globals_dict["__snail_normalize_sources"] = __snail_normalize_sources
    globals_dict["__snail_awk_field_separators"] = None
    globals_dict["__snail_awk_include_whitespace"] = False

    for helper_name, getter_name in _INSTALL_EAGER_HELPER_REGISTRY.items():
        globals_dict[helper_name] = _GETTERS[getter_name]()
