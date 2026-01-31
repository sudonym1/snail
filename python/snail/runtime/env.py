from __future__ import annotations

import os


class EnvMap:
    __slots__ = ("_env",)

    def __init__(self, env=None) -> None:
        self._env = os.environ if env is None else env

    def __fallback__(self) -> str:
        return ""

    def _lookup(self, key):
        try:
            return self._env[key]
        except KeyError as exc:
            exc.__fallback__ = self.__fallback__
            raise

    def __getitem__(self, key):
        return self._lookup(key)

    def __getattr__(self, name):
        return self._lookup(name)
