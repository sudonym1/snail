from __future__ import annotations

import subprocess


class SubprocessCapture:
    def __init__(self, cmd: str) -> None:
        self.cmd = cmd

    def __call__(self, input_data=None):
        try:
            if input_data is None:
                completed = subprocess.run(
                    self.cmd,
                    shell=True,
                    check=True,
                    text=True,
                    stdout=subprocess.PIPE,
                )
            else:
                if not isinstance(input_data, (str, bytes)):
                    input_data = str(input_data)
                completed = subprocess.run(
                    self.cmd,
                    shell=True,
                    check=True,
                    text=True,
                    input=input_data,
                    stdout=subprocess.PIPE,
                )
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
            if input_data is None:
                subprocess.run(self.cmd, shell=True, check=True)
            else:
                if not isinstance(input_data, (str, bytes)):
                    input_data = str(input_data)
                subprocess.run(
                    self.cmd,
                    shell=True,
                    check=True,
                    text=True,
                    input=input_data,
                )
            return 0
        except subprocess.CalledProcessError as exc:

            def __fallback(exc=exc):
                return exc.returncode

            exc.__fallback__ = __fallback
            raise
