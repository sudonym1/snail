from __future__ import annotations

import json as _json
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
    """Wrapper for json() to support pipeline operator without blocking stdin."""

    def __pipeline__(self, input_data):
        return json(input_data)

    def __structured__(self, query: str):
        data = json(_sys.stdin)
        return data.__structured__(query)

    def __repr__(self) -> str:
        data = json(_sys.stdin)
        return repr(data)


def json(input_data=None):
    """Parse JSON from various input sources."""
    if input_data is None:
        return JsonPipelineWrapper()

    if isinstance(input_data, str):
        try:
            data = _json.loads(input_data)
        except _json.JSONDecodeError:
            with open(input_data, "r", encoding="utf-8") as handle:
                data = _json.load(handle)
    elif hasattr(input_data, "read"):
        content = input_data.read()
        if isinstance(content, bytes):
            content = content.decode("utf-8")
        data = _json.loads(content)
    elif isinstance(input_data, (dict, list, int, float, bool)) or input_data is None:
        data = input_data
    else:
        raise TypeError(
            f"json() input must be JSON-compatible, got {type(input_data).__name__}"
        )

    return JsonObject(data)
