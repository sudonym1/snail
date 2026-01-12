from __future__ import annotations

import re


def regex_search(value, pattern):
    return re.search(pattern, value)


def regex_compile(pattern):
    return re.compile(pattern)
