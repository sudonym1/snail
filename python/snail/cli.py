from __future__ import annotations

import argparse
import ast
import builtins
import os
import sys
import traceback
from pathlib import Path

from . import __build_info__, __version__, compile_ast, exec


def _build_parser() -> argparse.ArgumentParser:
    return argparse.ArgumentParser(
        prog="snail",
        description="Snail programming language interpreter",
        usage="snail [options] -f <file> [args]...\n       snail [options] <code> [args]...",
        add_help=True,
    )


def _display_filename(filename: str) -> str:
    if filename.startswith("snail:"):
        return filename
    return f"snail:{filename}"


def _trim_internal_prefix(
    stack: traceback.StackSummary,
    internal_files: set[str],
) -> None:
    if not stack:
        return
    trim_count = 0
    for frame in stack:
        filename = frame.filename
        if filename.startswith("snail:"):
            break
        if filename in internal_files:
            trim_count += 1
            continue
        if os.path.isabs(filename) and os.path.abspath(filename) in internal_files:
            trim_count += 1
            continue
        break
    if 0 < trim_count < len(stack):
        del stack[:trim_count]


def _trim_traceback_exception(
    tb_exc: traceback.TracebackException,
    internal_files: set[str],
) -> None:
    _trim_internal_prefix(tb_exc.stack, internal_files)
    cause = getattr(tb_exc, "__cause__", None)
    if cause is not None:
        _trim_traceback_exception(cause, internal_files)
    context = getattr(tb_exc, "__context__", None)
    if context is not None:
        _trim_traceback_exception(context, internal_files)
    for group_exc in getattr(tb_exc, "exceptions", ()) or ():
        _trim_traceback_exception(group_exc, internal_files)


def _install_trimmed_excepthook() -> None:
    entrypoint = os.path.abspath(sys.argv[0])
    cli_path = os.path.abspath(__file__)
    internal_files = {entrypoint, cli_path}
    original_excepthook = sys.excepthook

    def _snail_excepthook(
        exc_type: type[BaseException],
        exc: BaseException,
        tb: object,
    ) -> None:
        if exc_type is KeyboardInterrupt:
            return original_excepthook(exc_type, exc, tb)
        tb_exc = traceback.TracebackException(
            exc_type,
            exc,
            tb,
            capture_locals=False,
        )
        _trim_traceback_exception(tb_exc, internal_files)
        try:
            import _colorize

            colorize = _colorize.can_colorize(file=sys.stderr)
        except Exception:
            colorize = hasattr(sys.stderr, "isatty") and sys.stderr.isatty()
        for line in tb_exc.format(colorize=colorize):
            sys.stderr.write(line)

    sys.excepthook = _snail_excepthook


def _format_version(version: str, build_info: dict[str, object] | None) -> str:
    display_version = version if version.startswith("v") else f"v{version}"
    if not build_info:
        return display_version
    git_rev = build_info.get("git_rev")
    if not git_rev:
        return display_version

    suffixes: list[str] = []
    if build_info.get("dirty"):
        suffixes.append("!dirty")
    if build_info.get("untagged"):
        suffixes.append("!untagged")

    if suffixes:
        return f"{display_version} ({git_rev}) {' '.join(suffixes)}"
    return f"{display_version} ({git_rev})"


def main(argv: list[str] | None = None) -> int:
    if argv is None:
        _install_trimmed_excepthook()

    parser = _build_parser()
    parser.add_argument("-f", dest="file", metavar="file")
    parser.add_argument("-a", "--awk", action="store_true")
    parser.add_argument("-P", "--no-print", action="store_true")
    parser.add_argument("-I", "--no-auto-import", action="store_true")
    parser.add_argument("--debug", action="store_true", help="Parse and compile, then print, do not run")
    parser.add_argument("-v", "--version", action="store_true")
    parser.add_argument("args", nargs=argparse.REMAINDER)

    namespace = parser.parse_args(argv)

    if namespace.version:
        print(_format_version(__version__, __build_info__))
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

    if namespace.debug:
        python_ast = compile_ast(
            source,
            mode=mode,
            auto_print=not namespace.no_print,
            filename=filename,
        )
        builtins.compile(python_ast, _display_filename(filename), "exec")
        print(ast.unparse(python_ast))
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
