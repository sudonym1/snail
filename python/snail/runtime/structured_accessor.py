from __future__ import annotations

import json as _json
import os as _os
import sys as _sys


def _append_transpiled_char(out: list[str], ch: str) -> None:
    if ch == "'":
        out.append("\\")
        out.append("'")
        return
    if ch == "\\":
        out.append("\\")
        out.append("\\")
        return
    out.append(ch)


def _transpile_jmespath_query(query: str) -> str:
    out: list[str] = []
    state = "normal"
    i = 0
    while i < len(query):
        ch = query[i]
        if state == "normal":
            if ch == "\\" and i + 1 < len(query) and query[i + 1] == '"':
                out.append('"')
                i += 2
                continue
            if ch == "'":
                state = "single"
                out.append(ch)
                i += 1
                continue
            if ch == "`":
                state = "backtick"
                out.append(ch)
                i += 1
                continue
            if ch == '"':
                state = "double"
                out.append("'")
                i += 1
                continue
            out.append(ch)
            i += 1
            continue

        if state == "single":
            out.append(ch)
            if ch == "\\" and i + 1 < len(query):
                out.append(query[i + 1])
                i += 2
                continue
            if ch == "'":
                state = "normal"
            i += 1
            continue

        if state == "backtick":
            out.append(ch)
            if ch == "\\" and i + 1 < len(query):
                out.append(query[i + 1])
                i += 2
                continue
            if ch == "`":
                state = "normal"
            i += 1
            continue

        if state == "double":
            if ch == '"':
                out.append("'")
                state = "normal"
                i += 1
                continue
            if ch == "\\" and i + 1 < len(query):
                nxt = query[i + 1]
                if nxt == '"':
                    _append_transpiled_char(out, '"')
                    i += 2
                    continue
                if nxt == "\\":
                    _append_transpiled_char(out, "\\")
                    i += 2
                    continue
                _append_transpiled_char(out, "\\")
                i += 1
                continue
            _append_transpiled_char(out, ch)
            i += 1

    return "".join(out)


def __snail_jmespath_query(query: str):
    """Create a callable that applies JMESPath query.

    Used by the $[query] syntax which lowers to __snail_jmespath_query(query).
    """

    import jmespath as _jmespath  # type: ignore[import-untyped]

    transpiled = _transpile_jmespath_query(query)

    def apply(data):
        return _jmespath.search(transpiled, data)

    return apply


def _parse_jsonl(content: str):
    lines = [line for line in content.splitlines() if line.strip()]
    if not lines:
        return []

    items = []
    for line in lines:
        try:
            items.append(_json.loads(line))
        except _json.JSONDecodeError as exc:
            raise _json.JSONDecodeError(
                f"Invalid JSONL line: {exc.msg}",
                line,
                exc.pos,
            ) from exc
    return items


def js(input_data=None):
    """Parse JSON from various input sources.

    Returns the parsed Python object (dict, list, etc.) directly.
    If called with no arguments, reads from stdin.
    """
    if input_data is None:
        try:
            is_tty = _sys.stdin.isatty()
        except Exception:
            is_tty = False
        if is_tty:
            raise ValueError('Missing input (see "snail --help")')
        input_data = _sys.stdin

    if input_data == "-":

        input_data = _sys.stdin

    if isinstance(input_data, str):
        try:
            return _json.loads(input_data)
        except _json.JSONDecodeError:
            if _os.path.exists(input_data):
                with open(input_data, "r", encoding="utf-8") as handle:
                    content = handle.read()
                try:
                    return _json.loads(content)
                except _json.JSONDecodeError:
                    return _parse_jsonl(content)
            else:
                return _parse_jsonl(input_data)
    elif hasattr(input_data, "read"):
        content = input_data.read()
        if isinstance(content, bytes):
            content = content.decode("utf-8")
        try:
            return _json.loads(content)
        except _json.JSONDecodeError:
            return _parse_jsonl(content)
    elif isinstance(input_data, (dict, list, int, float, bool)) or input_data is None:
        return input_data
    else:
        raise TypeError(
            f"js() input must be JSON-compatible, got {type(input_data).__name__}"
        )
