from __future__ import annotations

import os
import sys

from . import __build_info__, compile_ast, exec

_USAGE = (
    "snail [options] -f <file> [args]...\n"
    "       snail [options] <code> [args]..."
)
_DESCRIPTION = "Snail programming language interpreter"


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
        for line in tb_exc.format(colorize=colorize):
            sys.stderr.write(line)

    sys.excepthook = _snail_excepthook


class _Args:
    def __init__(self) -> None:
        self.file: str | None = None
        self.awk = False
        self.no_print = False
        self.no_auto_import = False
        self.debug = False
        self.version = False
        self.help = False
        self.begin_code: list[str] = []
        self.end_code: list[str] = []
        self.args: list[str] = []


def _print_help(file=sys.stdout) -> None:
    print(f"usage: {_USAGE}", file=file)
    print("", file=file)
    print(_DESCRIPTION, file=file)
    print("", file=file)
    print("options:", file=file)
    print("  -f <file>               read Snail source from file", file=file)
    print("  -a, --awk               awk mode", file=file)
    print("  -b <code>               begin block code (awk mode only, repeatable)", file=file)
    print("  -e <code>               end block code (awk mode only, repeatable)", file=file)
    print("  -P, --no-print          disable auto-print of last expression", file=file)
    print("  -I, --no-auto-import    disable auto-imports", file=file)
    print("  --debug                 parse and compile, then print, do not run", file=file)
    print("  -v, --version           show version and exit", file=file)
    print("  -h, --help              show this help message and exit", file=file)


def _parse_args(argv: list[str]) -> _Args:
    args = _Args()
    idx = 0
    code_found = False
    while idx < len(argv):
        token = argv[idx]
        if token == "--":
            args.args = argv[idx + 1 :]
            return args
        if token == "-" or not token.startswith("-"):
            if code_found:
                # Already found code, rest are args
                args.args = argv[idx:]
                return args
            # This is the code, continue parsing for -b/-e after
            args.args = [token]
            code_found = True
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
        if token == "-f":
            if idx + 1 >= len(argv):
                raise ValueError("option -f requires an argument")
            args.file = argv[idx + 1]
            idx += 2
            continue
        if token == "-b":
            if idx + 1 >= len(argv):
                raise ValueError("option -b requires an argument")
            args.begin_code.append(argv[idx + 1])
            idx += 2
            continue
        if token == "-e":
            if idx + 1 >= len(argv):
                raise ValueError("option -e requires an argument")
            args.end_code.append(argv[idx + 1])
            idx += 2
            continue
        raise ValueError(f"unknown option: {token}")
    return args


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


def _get_version() -> str:
    from . import __version__ as version

    return version


def main(argv: list[str] | None = None) -> int:
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

    # Validate -b/-e only with --awk mode
    if (namespace.begin_code or namespace.end_code) and not namespace.awk:
        print("error: -b and -e options require --awk mode", file=sys.stderr)
        return 2

    mode = "awk" if namespace.awk else "snail"

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
        print(ast.unparse(python_ast))
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
