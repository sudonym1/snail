from __future__ import annotations

import subprocess


class SnailExitStatus(int):
    """Subprocess exit status with shell-like truthiness."""

    def __new__(cls, rc: int) -> "SnailExitStatus":
        return super().__new__(cls, rc)

    def __bool__(self) -> bool:
        return int(self) == 0

    @property
    def rc(self) -> int:
        return int(self)


def _run_subprocess(cmd: str, input_data=None, *, capture: bool):
    if isinstance(input_data, bytes):
        input_data = input_data.decode()
    elif input_data is not None and not isinstance(input_data, str):
        input_data = str(input_data)

    kwargs: dict[str, object] = {
        "shell": True,
        "check": True,
        "text": True,
    }

    if capture:
        kwargs["stdout"] = subprocess.PIPE

    if input_data is not None:
        kwargs["input"] = input_data

    return subprocess.run(cmd, **kwargs)  # type: ignore[call-overload]


class SubprocessCapture:
    def __init__(self, cmd: str) -> None:
        self.cmd = cmd

    def __call__(self, input_data=None):
        try:
            completed = _run_subprocess(self.cmd, input_data, capture=True)
            return completed.stdout.rstrip("\n")
        except subprocess.CalledProcessError as exc:

            def __fallback(exc=exc):
                raise exc

            exc.__fallback__ = __fallback
            raise


class SubprocessStatus:
    def __init__(self, cmd: str) -> None:
        self.cmd = cmd

    def __call__(self, input_data=None):
        try:
            _run_subprocess(self.cmd, input_data, capture=False)
            return SnailExitStatus(0)
        except subprocess.CalledProcessError as exc:

            def __fallback(exc=exc):
                return SnailExitStatus(exc.returncode)

            exc.__fallback__ = __fallback
            raise
