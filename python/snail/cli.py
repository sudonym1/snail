from __future__ import annotations

import os
import sys
from typing import Optional

from . import __build_info__, compile_ast, exec

_USAGE = (
    "snail [options] -f <file> [args]...\n"
    "       snail [options] <code> [args]..."
)
_DESCRIPTION = "Snail programming language interpreter"
_BOOLEAN_FLAGS = frozenset("amPIvh")
_VALUE_FLAGS = frozenset("fbe")


def _display_filename(filename: str) -> str:
    if filename.startswith("snail:"):
        return filename
    return f"snail:{filename}"


def _trim_internal_prefix(stack, internal_files: set[str]) -> None:
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


def _trim_traceback_exception(tb_exc, internal_files: set[str]) -> None:
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
        import traceback

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
        try:
            formatted = tb_exc.format(colorize=colorize)
        except TypeError:
            formatted = tb_exc.format()
        for line in formatted:
            sys.stderr.write(line)

    sys.excepthook = _snail_excepthook


class _Args:
    def __init__(self) -> None:
        self.file: Optional[str] = None
        self.awk = False
        self.map = False
        self.no_print = False
        self.no_auto_import = False
        self.debug = False
        self.debug_snail_ast = False
        self.version = False
        self.help = False
        self.begin_code: list[str] = []
        self.end_code: list[str] = []
        self.args: list[str] = []


def _print_help(file=None) -> None:
    if file is None:
        file = sys.stdout
    print(f"usage: {_USAGE}", file=file)
    print("", file=file)
    print(_DESCRIPTION, file=file)
    print("", file=file)
    print("options:", file=file)
    print("  -f <file>               read Snail source from file", file=file)
    print("  -a, --awk               awk mode", file=file)
    print("  -m, --map               map mode (process files one at a time)", file=file)
    print(
        "  -b, --begin <code>       begin block code (awk/map mode, repeatable)",
        file=file,
    )
    print(
        "  -e, --end <code>         end block code (awk/map mode, repeatable)",
        file=file,
    )
    print(
        "  -P, --no-print          disable auto-print of implicit return value",
        file=file,
    )
    print("  -I, --no-auto-import    disable auto-imports", file=file)
    print("  --debug                 parse and compile, then print, do not run", file=file)
    print("  --debug-snail-ast       parse and print Snail AST, do not run", file=file)
    print("  -v, --version           show version and exit", file=file)
    print("  -h, --help              show this help message and exit", file=file)


def _expand_short_options(argv: list[str]) -> list[str]:
    expanded: list[str] = []
    idx = 0
    while idx < len(argv):
        token = argv[idx]
        if token == "--":
            expanded.append(token)
            expanded.extend(argv[idx + 1 :])
            return expanded
        if token == "-" or not token.startswith("-") or token.startswith("--"):
            expanded.append(token)
            idx += 1
            continue
        if len(token) == 2:
            expanded.append(token)
            idx += 1
            continue

        flags = token[1:]
        pos = 0
        while pos < len(flags):
            flag = flags[pos]
            if flag in _BOOLEAN_FLAGS:
                expanded.append(f"-{flag}")
                pos += 1
                continue
            if flag in _VALUE_FLAGS:
                remainder = flags[pos + 1 :]
                if not remainder:
                    expanded.append(f"-{flag}")
                    pos += 1
                    continue
                if all(
                    ch in _BOOLEAN_FLAGS or ch in _VALUE_FLAGS for ch in remainder
                ):
                    raise ValueError(
                        f"option -{flag} requires an argument and must be last in a "
                        "combined flag group"
                    )
                expanded.append(f"-{flag}")
                expanded.append(remainder)
                pos = len(flags)
                break
            raise ValueError(f"unknown option: -{flag}")
        idx += 1
    return expanded


def _parse_args(argv: list[str]) -> _Args:
    argv = _expand_short_options(argv)
    args = _Args()
    idx = 0
    code_found = False
    while idx < len(argv):
        token = argv[idx]
        if token == "--":
            args.args = argv[idx + 1 :]
            return args
        if token == "-" or not token.startswith("-"):
            if not code_found:
                # This is the code (or the first arg when -f is used)
                args.args = [token]
                code_found = True
            else:
                args.args.append(token)
            idx += 1
            continue
        if token in ("-h", "--help"):
            args.help = True
            return args
        if token in ("-v", "--version"):
            args.version = True
            idx += 1
            continue
        if token in ("-a", "--awk"):
            args.awk = True
            idx += 1
            continue
        if token in ("-m", "--map"):
            args.map = True
            idx += 1
            continue
        if token in ("-P", "--no-print"):
            args.no_print = True
            idx += 1
            continue
        if token in ("-I", "--no-auto-import"):
            args.no_auto_import = True
            idx += 1
            continue
        if token == "--debug":
            args.debug = True
            idx += 1
            continue
        if token == "--debug-snail-ast":
            args.debug_snail_ast = True
            idx += 1
            continue
        if token == "-f":
            if idx + 1 >= len(argv):
                raise ValueError("option -f requires an argument")
            args.file = argv[idx + 1]
            code_found = True
            idx += 2
            continue
        if token in ("-b", "--begin"):
            if idx + 1 >= len(argv):
                raise ValueError(f"option {token} requires an argument")
            args.begin_code.append(argv[idx + 1])
            idx += 2
            continue
        if token in ("-e", "--end"):
            if idx + 1 >= len(argv):
                raise ValueError(f"option {token} requires an argument")
            args.end_code.append(argv[idx + 1])
            idx += 2
            continue
        raise ValueError(f"unknown option: {token}")
    return args


def _format_version(version: str, build_info: Optional[dict[str, object]]) -> str:
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


def _get_version() -> str:
    from . import __version__ as version

    return version


def main(argv: Optional[list[str]] = None) -> int:
    if argv is None:
        _install_trimmed_excepthook()
        argv = sys.argv[1:]

    try:
        namespace = _parse_args(argv)
    except ValueError as exc:
        _print_help(file=sys.stderr)
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if namespace.help:
        _print_help()
        return 0
    if namespace.version:
        print(_format_version(_get_version(), __build_info__))
        return 0

    # Validate --awk and --map are mutually exclusive
    if namespace.awk and namespace.map:
        print("error: --awk and --map cannot be used together", file=sys.stderr)
        return 2

    # Validate -b/--begin and -e/--end only with --awk or --map mode
    if (namespace.begin_code or namespace.end_code) and not (namespace.awk or namespace.map):
        print(
            "error: -b/--begin and -e/--end options require --awk or --map mode",
            file=sys.stderr,
        )
        return 2

    mode = "map" if namespace.map else ("awk" if namespace.awk else "snail")

    if namespace.file:
        from pathlib import Path

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

    if namespace.debug_snail_ast:
        from . import parse_ast

        snail_ast = parse_ast(
            source,
            mode=mode,
            filename=filename,
            begin_code=namespace.begin_code,
            end_code=namespace.end_code,
        )
        print(snail_ast)
        return 0

    if namespace.debug:
        import ast
        import builtins

        python_ast = compile_ast(
            source,
            mode=mode,
            auto_print=not namespace.no_print,
            filename=filename,
            begin_code=namespace.begin_code,
            end_code=namespace.end_code,
        )
        builtins.compile(python_ast, _display_filename(filename), "exec")
        try:
            output = ast.unparse(python_ast)
        except AttributeError:
            import astunparse

            output = astunparse.unparse(python_ast).rstrip("\n")
        print(output)
        return 0

    return exec(
        source,
        argv=args,
        mode=mode,
        auto_print=not namespace.no_print,
        auto_import=not namespace.no_auto_import,
        filename=filename,
        begin_code=namespace.begin_code,
        end_code=namespace.end_code,
    )


if __name__ == "__main__":
    raise SystemExit(main())
