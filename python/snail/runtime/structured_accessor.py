from __future__ import annotations

import json as _json
import os as _os
import sys as _sys

from ..vendor import jmespath


class StructuredAccessor:
    def __init__(self, query: str) -> None:
        self.query = query

    def __pipeline__(self, obj):
        if not hasattr(obj, "__structured__"):
            raise TypeError(
                "Pipeline target must implement __structured__, "
                f"got {type(obj).__name__}"
            )
        return obj.__structured__(self.query)


class JsonObject:
    def __init__(self, data) -> None:
        self.data = data

    def __structured__(self, query: str):
        return jmespath.search(query, self.data)

    def __repr__(self) -> str:
        return _json.dumps(self.data, indent=2)


class JsonPipelineWrapper:
    """Wrapper for js() to support pipeline operator without blocking stdin."""

    def __pipeline__(self, input_data):
        return js(input_data)

    def __structured__(self, query: str):
        data = js(_sys.stdin)
        return data.__structured__(query)

    def __repr__(self) -> str:
        data = js(_sys.stdin)
        return repr(data)


class JoinPipelineWrapper:
    def __init__(self, separator: str) -> None:
        self.separator = separator

    def __pipeline__(self, input_data):
        return join(self.separator, input_data)


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


def join(separator: str = "\n", input_data=None):
    if input_data is None:
        return JoinPipelineWrapper(separator)

    return separator.join(str(item) for item in input_data)


def js(input_data=None):
    """Parse JSON from various input sources."""
    if input_data is None:
        return JsonPipelineWrapper()

    if isinstance(input_data, str):
        try:
            data = _json.loads(input_data)
        except _json.JSONDecodeError:
            if _os.path.exists(input_data):
                with open(input_data, "r", encoding="utf-8") as handle:
                    content = handle.read()
                try:
                    data = _json.loads(content)
                except _json.JSONDecodeError:
                    data = _parse_jsonl(content)
            else:
                data = _parse_jsonl(input_data)
    elif hasattr(input_data, "read"):
        content = input_data.read()
        if isinstance(content, bytes):
            content = content.decode("utf-8")
        try:
            data = _json.loads(content)
        except _json.JSONDecodeError:
            data = _parse_jsonl(content)
    elif isinstance(input_data, (dict, list, int, float, bool)) or input_data is None:
        data = input_data
    else:
        raise TypeError(
            f"js() input must be JSON-compatible, got {type(input_data).__name__}"
        )

    return JsonObject(data)
