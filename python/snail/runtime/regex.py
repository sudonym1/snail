from __future__ import annotations

import re


def regex_search(value, pattern):
    match = re.search(pattern, value)
    if match is None:
        return ()
    return (match.group(0),) + match.groups()


def regex_compile(pattern):
    return re.compile(pattern)
