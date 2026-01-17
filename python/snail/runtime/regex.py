from __future__ import annotations

import re


class SnailRegex:
    def __init__(self, pattern: str) -> None:
        self.pattern = pattern
        self._regex = re.compile(pattern)

    def search(self, value):
        return regex_search(value, self._regex)

    def __contains__(self, value):
        return bool(self.__snail_contains__(value))

    def __snail_contains__(self, value):
        return self.search(value)

    def __repr__(self) -> str:
        return f"/{self.pattern}/"


def regex_search(value, pattern):
    if isinstance(pattern, SnailRegex):
        return pattern.search(value)
    if hasattr(pattern, "search"):
        match = pattern.search(value)
    else:
        match = re.search(pattern, value)
    if match is None:
        return ()
    return (match.group(0),) + match.groups()


def regex_compile(pattern):
    return SnailRegex(pattern)
