from __future__ import annotations

import argparse
import sys
from pathlib import Path

from . import __version__, exec, parse


def _build_parser() -> argparse.ArgumentParser:
    return argparse.ArgumentParser(
        prog="snail",
        description="Snail programming language interpreter",
        usage="snail [options] -f <file> [args]...\n       snail [options] <code> [args]...",
        add_help=True,
    )


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    parser.add_argument("-f", dest="file", metavar="file")
    parser.add_argument("-a", "--awk", action="store_true")
    parser.add_argument("-P", "--no-print", action="store_true")
    parser.add_argument("-I", "--no-auto-import", action="store_true")
    parser.add_argument("--parse-only", action="store_true")
    parser.add_argument("-v", "--version", action="store_true")
    parser.add_argument("args", nargs=argparse.REMAINDER)

    namespace = parser.parse_args(argv)

    if namespace.version:
        print(__version__)
        return 0

    mode = "awk" if namespace.awk else "snail"

    if namespace.file:
        path = Path(namespace.file)
        try:
            source = path.read_text()
        except OSError as exc:
            print(f"failed to read {path}: {exc}", file=sys.stderr)
            return 1
        filename = str(path)
        args = [filename, *namespace.args]
    else:
        if not namespace.args:
            print("no input provided", file=sys.stderr)
            return 1
        source = namespace.args[0]
        filename = "<cmd>"
        args = ["--", *namespace.args[1:]]

    if namespace.parse_only:
        parse(source, mode=mode, filename=filename)
        return 0

    return exec(
        source,
        argv=args,
        mode=mode,
        auto_print=not namespace.no_print,
        auto_import=not namespace.no_auto_import,
        filename=filename,
    )


if __name__ == "__main__":
    raise SystemExit(main())
