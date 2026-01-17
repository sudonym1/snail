from __future__ import annotations

import json as _json
import os as _os
import sys as _sys

def __snail_jmespath_query(query: str):
    """Create a callable that applies JMESPath query.

    Used by the $[query] syntax which lowers to __snail_jmespath_query(query).
    """

    import jmespath as _jmespath

    def apply(data):
        return _jmespath.search(query, data)

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
