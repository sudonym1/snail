from __future__ import annotations

import ast
import importlib
import importlib.util
import io
import os
import re
import shlex
import subprocess
import sys
import traceback
from pathlib import Path
from typing import Iterator, Optional

import pytest

ROOT = Path(__file__).resolve().parents[2]
PYTHON_DIR = ROOT / "python"
sys.path.insert(0, str(PYTHON_DIR))

if importlib.util.find_spec("snail._native") is None:
    pytest.skip("snail extension not built", allow_module_level=True)

snail = importlib.import_module("snail")
main = importlib.import_module("snail.cli").main

README_SNIPPET_PREAMBLE = """
def risky(*args, fail=False) { if fail { raise Exception(fail) } else { return args } }
def fetch_url(x) { return None }
def greet(*args, **kwargs) { print(*args) }
name = "world"
bad_email = "bad@@email"
phone = "867-5309"
my_bashvar = 123
"""


def _ensure_readme_map_file(tmp_path: Path) -> Path:
    map_file = tmp_path / "file1"
    map_file.write_text("readme map input\n")
    return map_file


def test_parse_only(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--debug", "x = 1"]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "x = 1"


def test_debug_snail_ast_basic(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--debug-snail-ast", "x = 1"]) == 0
    captured = capsys.readouterr()
    assert "Program" in captured.out
    assert "Assign" in captured.out


def test_debug_python_ast_basic(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--debug-python-ast", "x = 1"]) == 0
    captured = capsys.readouterr()
    assert "Module" in captured.out
    assert "Assign" in captured.out


def test_debug_snail_ast_awk(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--debug-snail-ast", "--awk", "/foo/"]) == 0
    captured = capsys.readouterr()
    assert "AwkProgram" in captured.out


def test_debug_snail_ast_map(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--debug-snail-ast", "--map", "print($src)"]) == 0
    captured = capsys.readouterr()
    assert "Program" in captured.out


def test_debug_snail_ast_map_begin_end_in_file(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert (
        main(
            [
                "--debug-snail-ast",
                "--map",
                "BEGIN { print(1) }\nprint($src)\nEND { print(2) }",
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert "begin_blocks" in captured.out
    assert "end_blocks" in captured.out


def test_debug_snail_ast_begin_end(capsys: pytest.CaptureFixture[str]) -> None:
    assert (
        main(
            [
                "--debug-snail-ast",
                "--awk",
                "-b",
                "x = 1",
                "-e",
                "print(x)",
                "/foo/",
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert "begin_blocks" in captured.out
    assert "end_blocks" in captured.out
    assert "Assign" in captured.out


def test_debug_snail_ast_file(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    script = tmp_path / "script.snail"
    script.write_text("x = 1")
    assert main(["--debug-snail-ast", "-f", str(script)]) == 0
    captured = capsys.readouterr()
    assert "Program" in captured.out


def test_debug_snail_ast_reports_parse_error(
    capsys: pytest.CaptureFixture[str],
) -> None:
    with pytest.raises(SyntaxError):
        main(["--debug-snail-ast", "x ="])


def test_parse_ast_api_basic() -> None:
    result = snail.parse_ast("x = 1")
    assert "Program" in result
    assert "Assign" in result


def test_parse_ast_api_snail_begin_end() -> None:
    result = snail.parse_ast(
        "print('body')",
        begin_code=["print('start')"],
        end_code=["print('done')"],
    )
    assert "begin_blocks" in result
    assert "end_blocks" in result
    assert "Program" in result


def test_parse_ast_api_map_begin_end() -> None:
    result = snail.parse_ast(
        "print($src)",
        mode="map",
        begin_code=["x = 1"],
        end_code=["print(x)"],
    )
    assert "begin_blocks" in result
    assert "end_blocks" in result
    assert "Assign" in result


@pytest.mark.parametrize(
    "api_name", ["compile", "compile_ast", "exec", "parse_ast", "parse"]
)
def test_native_api_unknown_mode_error(api_name: str) -> None:
    api = getattr(snail, api_name)
    with pytest.raises(RuntimeError, match=r"unknown mode: bad"):
        api("x = 1", mode="bad")


def test_exec_api_system_exit_none_returns_zero() -> None:
    assert snail.exec("raise SystemExit()", auto_print=False) == 0


def test_exec_api_system_exit_int_returns_code() -> None:
    assert snail.exec("raise SystemExit(3)", auto_print=False) == 3


def test_exec_api_system_exit_non_int_returns_one() -> None:
    assert snail.exec("raise SystemExit('boom')", auto_print=False) == 1


@pytest.mark.parametrize(
    ("mode", "source", "cli_begin", "file_begin", "file_end", "cli_end"),
    [
        (
            "snail",
            "BEGIN { print('snail-file-begin') }\nprint('body')\nEND { print('snail-file-end') }",
            "snail-cli-begin",
            "snail-file-begin",
            "snail-file-end",
            "snail-cli-end",
        ),
        (
            "map",
            "BEGIN { print('map-file-begin') }\nprint($src)\nEND { print('map-file-end') }",
            "map-cli-begin",
            "map-file-begin",
            "map-file-end",
            "map-cli-end",
        ),
    ],
)
def test_parse_ast_api_begin_end_merge_order(
    mode: str,
    source: str,
    cli_begin: str,
    file_begin: str,
    file_end: str,
    cli_end: str,
) -> None:
    result = snail.parse_ast(
        source,
        mode=mode,
        begin_code=[f"print('{cli_begin}')"],
        end_code=[f"print('{cli_end}')"],
    )
    assert "begin_blocks" in result
    assert "end_blocks" in result
    assert result.index(f'value: "{cli_begin}"') < result.index(f'value: "{file_begin}"')
    assert result.index(f'value: "{file_end}"') < result.index(f'value: "{cli_end}"')


@pytest.mark.parametrize(
    ("mode", "source"),
    [
        ("snail", "print('body')"),
        ("map", "print($src)"),
    ],
)
def test_parse_ast_api_ignores_whitespace_only_begin_end_code(
    mode: str, source: str
) -> None:
    result = snail.parse_ast(
        source,
        mode=mode,
        begin_code=["   ", "\n\t", "\n   \n"],
        end_code=["\n", " \t ", "\n\n"],
    )
    assert result.lstrip().startswith("Program {")
    assert "begin_blocks" not in result
    assert "end_blocks" not in result


def test_compile_api_traceback_uses_explicit_filename() -> None:
    filename = "compile-api-trace.snail"
    code = snail.compile("raise ValueError('boom')", filename=filename)

    with pytest.raises(ValueError) as excinfo:
        exec(code, {})

    filenames = [
        frame.filename for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert f"snail:{filename}" in filenames


def test_traceback_highlights_inline_snail() -> None:
    with pytest.raises(NameError) as excinfo:
        main(["x"])
    filenames = [
        frame.filename for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert "snail:<cmd>" in filenames


def test_traceback_highlights_file_snail(tmp_path: Path) -> None:
    script = tmp_path / "script.snail"
    script.write_text("x\n")
    with pytest.raises(NameError) as excinfo:
        main(["-f", str(script)])
    filenames = [
        frame.filename for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert f"snail:{script}" in filenames


def test_traceback_highlights_library_snail() -> None:
    import snail

    with pytest.raises(NameError) as excinfo:
        snail.exec("x", filename="lib.snail")
    filenames = [
        frame.filename for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert "snail:lib.snail" in filenames


@pytest.fixture(autouse=True)
def _stdin_devnull(monkeypatch: pytest.MonkeyPatch) -> Iterator[None]:
    with open("/dev/null", "r") as handle:
        monkeypatch.setattr(sys, "stdin", handle)
        yield


def set_stdin(
    monkeypatch: pytest.MonkeyPatch, text: str, is_tty: bool | None = None
) -> None:
    stdin = io.StringIO(text)
    monkeypatch.setattr(sys, "stdin", stdin)
    if is_tty is not None:
        monkeypatch.setattr(sys.stdin, "isatty", lambda: is_tty)


def run_cli(
    capsys: pytest.CaptureFixture[str], args: list[str] | tuple[str, ...]
) -> tuple[int, pytest.CaptureResult[str]]:
    result = main(list(args))
    return result, capsys.readouterr()


def test_no_print(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["-P", "1 + 1"]) == 0
    captured = capsys.readouterr()
    assert captured.out == ""


def test_inline_print(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["print('hi')"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "hi\n"


def test_implicit_return_function(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def add(a, b) {",
            "    a + b",
            "}",
            "print(add(1, 2))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "3"


def test_lambda_semicolon_disables_implicit_return(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "def f { 2; }",
            "g = def { 2; }",
            "print(f())",
            "print(g())",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == ["None", "None"]


def test_implicit_return_if_else_requires_return(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "def pick(flag) {",
            "    if flag { 1 } else { 2 }",
            "}",
            "print(pick(True))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "None"


def test_auto_print_uses_returned_value(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def add(a, b) {",
            "    a + b",
            "}",
            "add(1, 2)",
        ]
    )
    assert main([script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "3"


def test_compact_try_default_none(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def boom() { raise ValueError('nope') }",
            "value = boom()?",
            "print(value is None)",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "True"


def test_generator_yield(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def counter(n) {",
            "    i = 0",
            "    while i < n {",
            "        yield i",
            "        i = i + 1",
            "    }",
            "}",
            "def chain() {",
            "    yield from counter(2)",
            "    yield 5",
            "}",
            "for value in chain() { print(value) }",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == ["0", "1", "5"]


def test_top_level_yield_rejected() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(["yield 1"])
    message = str(excinfo.value)
    assert "yield" in message
    assert "function" in message


def test_def_expr_basic(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "adder = def(x, y) { x + y }",
            "print(adder(2, 3))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "5"


def test_def_expr_block_body(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "twice = def(x) { y = x + 1; y * 2 }",
            "print(twice(3))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "8"


def test_def_expr_closure(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def make_adder(n) { return def(x) { x + n } }",
            "add_five = make_adder(5)",
            "print(add_five(7))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "12"


def test_file_args(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    script = tmp_path / "script.snail"
    script.write_text("import sys\nprint(sys.argv[1])\n")
    assert main(["-f", str(script), "arg"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "arg\n"


def test_jsonl_file(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    jsonl = tmp_path / "data.jsonl"
    jsonl.write_text('{"name": "Ada"}\n{"name": "Lin"}\n')
    script = tmp_path / "script.snail"
    script.write_text(f"data = js({str(jsonl)!r})\nprint(data | $[[*].name])\n")
    assert main(["-f", str(script)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "['Ada', 'Lin']\n"


def test_js_dash_reads_stdin(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO('{"name": "Ada"}'))
    script = 'data = js("-")\nprint(data["name"])\n'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "Ada\n"


def test_js_requires_input_when_stdin_is_tty(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO(""))
    monkeypatch.setattr(sys.stdin, "isatty", lambda: True)
    with pytest.raises(ValueError) as excinfo:
        main(["-P", "js()"])
    assert 'Missing input (see "snail --help")' in str(excinfo.value)


def test_js_does_not_require_input_when_stdin_is_not_a_tty(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO(""))
    monkeypatch.setattr(sys.stdin, "isatty", lambda: False)
    result = main(["-P", "js()"])
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out == ""


def test_jmespath_double_quotes_string_literal(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            'data = js(%{"items": [%{"ifname": "eth0"}, %{"ifname": "wlan0"}]})',
            'print(data | $[items[?ifname=="eth0"].ifname])',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "['eth0']\n"


def test_jmespath_double_quotes_single_quote_escape(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            'data = js(%{"items": [%{"name": "O\'Connor"}, %{"name": "Ada"}]})',
            'print(data | $[items[?name=="O\'Connor"].name])',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == '["O\'Connor"]\n'


def test_jmespath_escaped_double_quotes_for_identifier(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            'data = js(%{"foo-bar": 1})',
            'print(data | $[\\"foo-bar\\"])',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "1\n"


def test_pipeline_placeholder(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def greet(name, suffix) { return name + suffix }",
            "print('Hi' | greet(_, '!'))",
            "print('Hi' | greet('Hello ', _))",
        ]
    )
    assert main([script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "Hi!\nHello Hi\n"


def test_placeholder_as_identifier(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["_ = 5\nprint(_ + 1)"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "6\n"


def test_if_let_destructure(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            'pair = ["user", "example.com"]',
            'if let [user, domain] = pair { print(domain) } else { print("no") }',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "example.com"


def test_if_let_guard(capsys: pytest.CaptureFixture[str]) -> None:
    script = 'if let x = 1; x == 2 { print("yes") } else { print("no") }'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "no"


def test_starred_destructuring(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "nums = [1, 2, 3]",
            "x, *xs = nums",
            "print(x)",
            "print(xs)",
            "if let [head, *tail] = nums { print(head); print(len(tail)) }",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "1\n[2, 3]\n1\n2\n"


def test_set_literals(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "nums = #{1, 2, 2, 3}",
            "empty = #{}",
            "print(len(nums))",
            "print(2 in nums)",
            "print(len(empty))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "3\nTrue\n0\n"


def test_dict_literals(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            'pairs = %{"a": 1, "b": 2}',
            "empty = %{}",
            'print(pairs["a"])',
            "print(len(empty))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "1\n0\n"


def test_while_let_destructure(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "def next_item(items, idx) {",
            "    if idx < len(items) { return items[idx] }",
            "    return None",
            "}",
            'items = [[1, "a"], [2, "b"]]',
            "i = 0",
            "while let [n, s] = next_item(items, i) {",
            "    print(s)",
            "    i = i + 1",
            "}",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "a\nb\n"


def test_regex_match_tuple(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            'm = "IJ" in /(I)(J)/',
            "print(m[0])",
            "print(m[1])",
            "print(m[2])",
            'print(len("xx" in /a/))',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "IJ\nI\nJ\n0\n"


def test_compiled_regex_object(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "pat = /ab(c)/",
            "print(pat.search('zabc')[1])",
            'm = "abc" in pat',
            "print(m[0])",
            "print(m[1])",
            'print(len("zzz" in pat))',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "c\nabc\nc\n0\n"


def test_contains_not_in(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "pat = /ab(c)/",
            "print('abc' not in pat)",
            "print('zzz' not in pat)",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "False\nTrue\n"


def test_contains_not_in_regex_literal(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "print('abc' not in /ab(c)/)",
            "print('zzz' not in /ab(c)/)",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "False\nTrue\n"


def test_chained_in_short_circuit(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "hits = [0]",
            "pat = /a/",
            "def bump() {",
            "    hits[0] = hits[0] + 1",
            "    return [pat]",
            "}",
            'print("a" in pat in bump())',
            "print(hits[0])",
            "hits[0] = 0",
            "pat = /z/",
            'print("a" in pat in bump())',
            "print(hits[0])",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "True\n1\n()\n0\n"


def test_chained_not_in_regex_short_circuit(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "hits = [0]",
            "pat = /a/",
            "def bump() {",
            "    hits[0] = hits[0] + 1",
            "    return [pat]",
            "}",
            'print("a" not in pat not in bump())',
            "print(hits[0])",
            "hits[0] = 0",
            "pat = /z/",
            'print("a" not in pat not in bump())',
            "print(hits[0])",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "False\n0\nFalse\n1\n"


def test_regex_search_helper_with_snailregex_object(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "pat = /a/",
            "print(__snail_regex_search('za', pat))",
            "print(__snail_regex_search('zz', pat))",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "('a',)\n()\n"


def test_contains_prefers_snail_hook_over_python_contains(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "class Hooked {",
            "    def __init__(self) {",
            "        self.snail_calls = 0",
            "        self.python_calls = 0",
            "    }",
            "    def __snail_contains__(self, value) {",
            "        self.snail_calls = self.snail_calls + 1",
            "        return [value]",
            "    }",
            "    def __contains__(self, value) {",
            "        self.python_calls = self.python_calls + 1",
            "        return False",
            "    }",
            "}",
            "obj = Hooked()",
            "print('x' in obj)",
            "print('x' not in obj)",
            "print(obj.snail_calls)",
            "print(obj.python_calls)",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "['x']\nFalse\n2\n0\n"


def test_augmented_assignment_and_increments(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "x = 5",
            "y = ++x",
            'print("pre", x, y)',
            "x = 5",
            "y = x++",
            'print("post", x, y)',
            "x = 5",
            "y = (x += 3)",
            'print("aug", x, y)',
            "class C {",
            "    def __init__(self) {",
            "        self.val = 1",
            "    }",
            "}",
            "c = C()",
            "y = ++c.val",
            'print("attr_pre", c.val, y)',
            "arr = [10]",
            "y = arr[0]++",
            'print("idx_post", arr[0], y)',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "pre 6 6\npost 6 5\naug 8 8\nattr_pre 2 2\nidx_post 11 10\n"


def test_increment_index_single_evaluation(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "arr = [10]",
            "calls = [0]",
            "def idx() {",
            "    calls[0] = calls[0] + 1",
            "    return 0",
            "}",
            "pre = ++arr[idx()]",
            'print("pre", arr[0], pre, calls[0])',
            "arr[0] = 10",
            "calls[0] = 0",
            "post = arr[idx()]++",
            'print("post", arr[0], post, calls[0])',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "pre 11 11 1\npost 11 10 1\n"


def test_increment_attr_chain_single_evaluation(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "class Cell {",
            "    def __init__(self, value) {",
            "        self.value = value",
            "    }",
            "}",
            "class Holder {",
            "    def __init__(self, value) {",
            "        self.cell = Cell(value)",
            "    }",
            "}",
            "holder = Holder(10)",
            "calls = [0]",
            "def get_holder() {",
            "    calls[0] = calls[0] + 1",
            "    return holder",
            "}",
            "pre = ++get_holder().cell.value",
            'print("pre", holder.cell.value, pre, calls[0])',
            "holder.cell.value = 10",
            "calls[0] = 0",
            "post = get_holder().cell.value++",
            'print("post", holder.cell.value, post, calls[0])',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "pre 11 11 1\npost 11 10 1\n"


def test_assignment_target_attr_index_chains(capsys: pytest.CaptureFixture[str]) -> None:
    script = "\n".join(
        [
            "class Cell {",
            "    def __init__(self, v) {",
            "        self.value = v",
            "    }",
            "}",
            "class Box {",
            "    def __init__(self) {",
            "        self.items = [Cell(0)]",
            "        self.meta = %{\"count\": 0}",
            "    }",
            "}",
            "box = Box()",
            "box.tag = 'ok'",
            "box.items[0].value = 2",
            "box.items[0].value += 3",
            "box.meta['count'] = 1",
            "box.meta['count'] += 2",
            "print(box.tag)",
            "print(box.items[0].value)",
            "print(box.meta['count'])",
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "ok\n5\n3\n"


def test_augmented_attr_index_single_evaluation(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "\n".join(
        [
            "class Box {",
            "    def __init__(self) {",
            "        self.value = 1",
            "    }",
            "}",
            "boxes = [Box()]",
            "target_calls = [0]",
            "idx_calls = [0]",
            "arr = [10]",
            "def get_target() {",
            "    target_calls[0] = target_calls[0] + 1",
            "    return 0",
            "}",
            "def get_idx() {",
            "    idx_calls[0] = idx_calls[0] + 1",
            "    return 0",
            "}",
            "boxes[get_target()].value += 4",
            "arr[get_idx()] += 5",
            'print("attr", boxes[0].value, target_calls[0])',
            'print("idx", arr[0], idx_calls[0])',
        ]
    )
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "attr 5 1\nidx 15 1\n"


def test_augmented_attr_getter_exception_skips_setter() -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class AttrGetterError:
        def __init__(self) -> None:
            self.setter_calls = 0

        @property
        def value(self):
            raise RuntimeError("attr getter boom")

        @value.setter
        def value(self, new) -> None:
            self.setter_calls += 1

    obj = AttrGetterError()
    with pytest.raises(RuntimeError, match="attr getter boom"):
        augmented.__snail_aug_attr(obj, "value", 1, "+")
    assert obj.setter_calls == 0


def test_augmented_index_getter_exception_skips_setter() -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class IndexGetterError:
        def __init__(self) -> None:
            self.setter_calls = 0

        def __getitem__(self, index):
            raise RuntimeError("index getter boom")

        def __setitem__(self, index, value) -> None:
            self.setter_calls += 1

    obj = IndexGetterError()
    with pytest.raises(RuntimeError, match="index getter boom"):
        augmented.__snail_aug_index(obj, 0, 1, "+")
    assert obj.setter_calls == 0


def test_augmented_setter_exception_called_once() -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class AttrSetterError:
        def __init__(self) -> None:
            self._value = 4
            self.setter_calls = 0

        @property
        def value(self):
            return self._value

        @value.setter
        def value(self, new) -> None:
            self.setter_calls += 1
            raise RuntimeError("attr setter boom")

    attr_obj = AttrSetterError()
    with pytest.raises(RuntimeError, match="attr setter boom"):
        augmented.__snail_aug_attr(attr_obj, "value", 1, "+")
    assert attr_obj.setter_calls == 1
    assert attr_obj._value == 4

    class IndexSetterError:
        def __init__(self) -> None:
            self.values = {0: 4}
            self.setter_calls = 0

        def __getitem__(self, index):
            return self.values[index]

        def __setitem__(self, index, value) -> None:
            self.setter_calls += 1
            raise RuntimeError("index setter boom")

    index_obj = IndexSetterError()
    with pytest.raises(RuntimeError, match="index setter boom"):
        augmented.__snail_aug_index(index_obj, 0, 1, "+")
    assert index_obj.setter_calls == 1
    assert index_obj.values[0] == 4


def test_augmented_attr_operator_error_no_mutation() -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class AttrTarget:
        def __init__(self) -> None:
            self._value = "text"
            self.setter_calls = 0

        @property
        def value(self):
            return self._value

        @value.setter
        def value(self, new) -> None:
            self.setter_calls += 1
            self._value = new

    obj = AttrTarget()
    with pytest.raises(TypeError):
        augmented.__snail_aug_attr(obj, "value", 1, "+")
    assert obj._value == "text"
    assert obj.setter_calls == 0


def test_augmented_index_operator_error_no_mutation() -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class IndexTarget:
        def __init__(self) -> None:
            self.values = {0: 9}
            self.setter_calls = 0

        def __getitem__(self, index):
            return self.values[index]

        def __setitem__(self, index, value) -> None:
            self.setter_calls += 1
            self.values[index] = value

    obj = IndexTarget()
    with pytest.raises(ZeroDivisionError):
        augmented.__snail_aug_index(obj, 0, 0, "/")
    assert obj.values[0] == 9
    assert obj.setter_calls == 0


@pytest.mark.parametrize(
    ("op", "expected"),
    [
        ("+", 7),
        ("-", 3),
        ("*", 10),
        ("/", 2.5),
        ("//", 2),
        ("%", 1),
        ("**", 25),
    ],
)
def test_augmented_ops_attr_coverage(op: str, expected: int | float) -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class AttrTarget:
        def __init__(self) -> None:
            self.value = 5

    obj = AttrTarget()
    result = augmented.__snail_aug_attr(obj, "value", 2, op)
    assert result == expected
    assert obj.value == expected


@pytest.mark.parametrize(
    ("op", "expected"),
    [
        ("+", 7),
        ("-", 3),
        ("*", 10),
        ("/", 2.5),
        ("//", 2),
        ("%", 1),
        ("**", 25),
    ],
)
def test_augmented_ops_index_coverage(op: str, expected: int | float) -> None:
    augmented = importlib.import_module("snail.runtime.augmented")

    class IndexTarget:
        def __init__(self) -> None:
            self.values = {0: 5}

        def __getitem__(self, index):
            return self.values[index]

        def __setitem__(self, index, value) -> None:
            self.values[index] = value

    obj = IndexTarget()
    result = augmented.__snail_aug_index(obj, 0, 2, op)
    assert result == expected
    assert obj.values[0] == expected


def test_combined_short_flags_awk(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    set_stdin(monkeypatch, "foo\n")
    result, captured = run_cli(capsys, ["-aP", "/foo/ { print($0) }"])
    assert result == 0
    assert captured.out == "foo\n"


@pytest.mark.parametrize(
    "attached_file_arg",
    [
        pytest.param(False, id="separate-file-arg"),
        pytest.param(True, id="attached-file-arg"),
    ],
)
def test_combined_short_flag_with_file_value(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    attached_file_arg: bool,
) -> None:
    script = tmp_path / "script.snail"
    script.write_text("/foo/ { print($0) }\n")
    set_stdin(monkeypatch, "foo\nbar\n")
    args = [f"-af{script}"] if attached_file_arg else ["-af", str(script)]
    result, captured = run_cli(capsys, args)
    assert result == 0
    assert captured.out == "foo\n"


def test_combined_short_flag_with_attached_field_separator(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    set_stdin(monkeypatch, "a,b\n")
    result, captured = run_cli(capsys, ["-aF,", "{ print($1) }"])
    assert result == 0
    assert captured.out == "a\n"


def test_combined_short_help(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["-ah"]) == 0
    captured = capsys.readouterr()
    assert "usage:" in captured.out


def test_version_prints_python_runtime(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--version"]) == 0
    captured = capsys.readouterr()
    lines = [line for line in captured.out.splitlines() if line.strip()]
    assert len(lines) >= 2
    python_line = lines[1]
    version = (
        f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    )
    assert python_line.startswith("Python ")
    assert version in python_line
    if sys.executable:
        assert os.path.abspath(sys.executable) in python_line


def test_value_flag_not_last_in_combination(
    capsys: pytest.CaptureFixture[str],
) -> None:
    result = main(["-fa"])
    assert result == 2
    captured = capsys.readouterr()
    assert "requires an argument" in captured.err


def test_unknown_flag_in_combination(capsys: pytest.CaptureFixture[str]) -> None:
    result = main(["-aX"])
    assert result == 2
    captured = capsys.readouterr()
    assert "unknown option: -X" in captured.err


@pytest.mark.parametrize(
    ("stdin_text", "is_tty", "expected_result", "expected_out", "expected_err"),
    [
        pytest.param("foo\nbar\n", None, 0, "foo\n", None, id="awk-has-input"),
        pytest.param(
            "",
            True,
            1,
            None,
            'Missing input (see "snail --help")',
            id="awk-tty-no-input",
        ),
        pytest.param("", False, 0, None, None, id="awk-nontty-no-input"),
    ],
)
def test_awk_input_handling(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    stdin_text: str,
    is_tty: bool | None,
    expected_result: int,
    expected_out: str | None,
    expected_err: str | None,
) -> None:
    set_stdin(monkeypatch, stdin_text, is_tty=is_tty)
    result, captured = run_cli(capsys, ["--awk", "/foo/ { print($0) }"])
    assert result == expected_result
    if expected_out is not None:
        assert captured.out == expected_out
    else:
        assert captured.out == ""
    if expected_err is not None:
        assert expected_err in captured.err
    else:
        assert captured.err == ""


def test_awk_file_dash_reads_stdin_when_stdin_is_tty(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = tmp_path / "script.snail"
    script.write_text("{ print($0) }\n")
    set_stdin(monkeypatch, "foo\nbar\n", is_tty=True)
    result, captured = run_cli(capsys, ["--awk", "-f", str(script), "-"])
    assert result == 0
    assert captured.out == "foo\nbar\n"
    assert captured.err == ""


def test_awk_src_current_file(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo\nbar\n"))
    assert main(["--awk", "{ print($src) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "-\n-\n"


def test_awk_field_separator_multiple_flags(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("a,b;c\n"))
    assert main(["--awk", "-F", ",", "-F", ";", "{ print($1, $2, $3) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "a b c\n"


def test_awk_field_separator_long_flags(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("a b/c\n"))
    assert (
        main(
            [
                "--awk",
                "--whitespace",
                "--field-separator",
                "/",
                "{ print($1, $2, $3) }",
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert captured.out == "a b c\n"


def test_awk_field_separator_whitespace_rules(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("/one/two/three\n"))
    assert (
        main(
            [
                "--awk",
                "--field-separator",
                "/",
                "{ print($1, $2, $3) }",
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert captured.out == "one two three\n"


def test_awk_field_separator_with_whitespace_flag(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(
        sys,
        "stdin",
        io.StringIO(
            "eth0             UP             172.20.223.220/20 fe80::215:5dff:fee5:ebb/64\n"
        ),
    )
    assert main(["--awk", "-W", "-F", "/", "{ print($1, $2, $3, $4, $5, $6) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "eth0 UP 172.20.223.220 20 fe80::215:5dff:fee5:ebb 64\n"


def test_awk_match_group_access(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo1\nfoo2\n"))
    assert main(["--awk", "/foo(\\d)/ { print($m.1) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "1\n2\n"


def test_awk_identifiers_require_awk_mode() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(["print($0)"])
    assert "--awk" in str(excinfo.value)


@pytest.mark.parametrize(
    "begin_flag",
    [
        pytest.param("-b", id="short-begin"),
        pytest.param("--begin", id="long-begin"),
    ],
)
def test_awk_begin_flags(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    begin_flag: str,
) -> None:
    set_stdin(monkeypatch, "line\n")
    result, captured = run_cli(
        capsys, ["--awk", begin_flag, "print('start')", "{ print($0) }"]
    )
    assert result == 0
    assert captured.out == "start\nline\n"


@pytest.mark.parametrize(
    "end_flag",
    [
        pytest.param("-e", id="short-end"),
        pytest.param("--end", id="long-end"),
    ],
)
def test_awk_end_flags(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    end_flag: str,
) -> None:
    set_stdin(monkeypatch, "line\n")
    result, captured = run_cli(
        capsys, ["--awk", end_flag, "print('done')", "{ print($0) }"]
    )
    assert result == 0
    assert captured.out == "line\ndone\n"


def test_awk_multiple_begin_end_flags(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("x\n"))
    assert (
        main(
            [
                "--awk",
                "-b",
                "print('b1')",
                "--begin",
                "print('b2')",
                "-e",
                "print('e1')",
                "--end",
                "print('e2')",
                "{ print($0) }",
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert captured.out == "b1\nb2\nx\ne1\ne2\n"


def test_awk_begin_end_interleaved_order(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("x\n"))
    # -b before code, -e after code
    assert (
        main(
            [
                "--awk",
                "-b",
                "print('start')",
                "{ print($0) }",
                "-e",
                "print('end')",
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert captured.out == "start\nx\nend\n"


def test_awk_begin_after_args(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    file_a = tmp_path / "a.txt"
    file_a.write_text("line\n")
    result = main(
        [
            "--awk",
            "{ print($0) }",
            str(file_a),
            "-b",
            "print('start')",
        ]
    )
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out == "start\nline\n"


def test_awk_begin_end_file_and_cli_order(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("x\n"))
    script = tmp_path / "file.snail"
    script.write_text(
        "BEGIN { print('file begin') }\n{ print($0) }\nEND { print('file end') }\n"
    )
    assert (
        main(
            [
                "--awk",
                "-b",
                "print('cli begin')",
                "-e",
                "print('cli end')",
                "-f",
                str(script),
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert captured.out.splitlines() == [
        "cli begin",
        "file begin",
        "x",
        "file end",
        "cli end",
    ]


def test_begin_end_regular_mode(capsys: pytest.CaptureFixture[str]) -> None:
    result = main(
        [
            "--begin",
            "print('start')",
            "--end",
            "print('done')",
            "print('body')",
        ]
    )
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out == "start\nbody\ndone\n"


def test_begin_end_regular_mode_file_and_cli_order(
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = "BEGIN { print('file begin') }\nprint('body')\nEND { print('file end') }"
    assert (
        main(
            [
                "--begin",
                "print('cli begin')",
                "--end",
                "print('cli end')",
                script,
            ]
        )
        == 0
    )
    captured = capsys.readouterr()
    assert captured.out.splitlines() == [
        "cli begin",
        "file begin",
        "body",
        "file end",
        "cli end",
    ]


def test_begin_end_regular_mode_oneliner_autoprint(
    capsys: pytest.CaptureFixture[str],
) -> None:
    result = main(["1 END { print('done') }"])
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == ["1", "done"]


# --- Tests for auto-import ---


@pytest.mark.parametrize(
    ("script", "check_mode", "expected"),
    [
        pytest.param("print(sys.version_info.major)", "isdigit", "", id="sys"),
        pytest.param("print(os.name)", "membership", ("posix", "nt"), id="os"),
        pytest.param('sys = "custom"\nprint(sys)', "equals", "custom", id="shadow"),
        pytest.param("print(Path('.').resolve())", "startswith", "/", id="path"),
    ],
)
def test_auto_import_enabled_variants(
    capsys: pytest.CaptureFixture[str],
    script: str,
    check_mode: str,
    expected: str | tuple[str, ...],
) -> None:
    result, captured = run_cli(capsys, ["-P", script])
    assert result == 0
    output = captured.out.strip()
    if check_mode == "isdigit":
        assert output.isdigit()
    elif check_mode == "membership":
        assert output in expected
    elif check_mode == "equals":
        assert output == expected
    else:
        assert output.startswith(expected)


@pytest.mark.parametrize(
    ("args", "expected_name"),
    [
        pytest.param(
            ["--no-auto-import", "print(sys.version)"], "sys", id="long-flag-sys"
        ),
        pytest.param(["-I", "print(os.name)"], "os", id="short-flag-os"),
        pytest.param(["-I", "print(Path('.'))"], "Path", id="short-flag-path"),
    ],
)
def test_auto_import_disabled_variants(
    args: list[str], expected_name: str
) -> None:
    with pytest.raises(NameError) as excinfo:
        main(args)
    assert expected_name in str(excinfo.value)


# --- Tests for $env ---


def test_env_map_reads(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setenv("SNAIL_ENV_TEST", "snail")
    script = "print($env.SNAIL_ENV_TEST)\nprint($env['SNAIL_ENV_TEST'])"
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out == "snail\nsnail\n"


def test_env_map_missing_raises(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("SNAIL_ENV_MISSING", raising=False)
    with pytest.raises(KeyError):
        snail.exec("print($env.SNAIL_ENV_MISSING)", auto_print=False)


def test_env_map_missing_fallback(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.delenv("SNAIL_ENV_MISSING", raising=False)
    assert main(["-P", "print(repr($env.SNAIL_ENV_MISSING?))"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "''\n"


def test_regex_search_custom_pattern_raises_propagates() -> None:
    runtime_regex = importlib.import_module("snail.runtime.regex")

    class RaisingPattern:
        def search(self, value):
            raise RuntimeError("custom search boom")

    with pytest.raises(RuntimeError, match="custom search boom"):
        runtime_regex.regex_search("abc", RaisingPattern())


def test_regex_search_custom_pattern_invalid_match_shape() -> None:
    runtime_regex = importlib.import_module("snail.runtime.regex")

    class BadMatch:
        def group(self, index):
            return "x"

    class InvalidPattern:
        def search(self, value):
            return BadMatch()

    with pytest.raises(AttributeError) as excinfo:
        runtime_regex.regex_search("abc", InvalidPattern())
    assert "groups" in str(excinfo.value)


def test_regex_match_invalid_pattern_raises() -> None:
    runtime_regex = importlib.import_module("snail.runtime.regex")
    with pytest.raises(re.error):
        runtime_regex.regex_search("abc", "(")


def test_regex_match_non_string_value_raises() -> None:
    runtime_regex = importlib.import_module("snail.runtime.regex")
    with pytest.raises(TypeError):
        runtime_regex.regex_search(123, re.compile("a"))


def test_runtime_helpers_installed_in_exec_globals() -> None:
    runtime = importlib.import_module("snail.runtime")
    globals_dict: dict[str, object] = {}
    runtime.install_helpers(globals_dict)

    helper_names = [
        "__snail_incr_attr",
        "__snail_incr_index",
        "__snail_aug_attr",
        "__snail_aug_index",
        "__snail_regex_search",
        "__snail_regex_compile",
        "__snail_contains__",
        "__snail_contains_not__",
    ]
    for name in helper_names:
        assert name in globals_dict
        assert callable(globals_dict[name])


def test_runtime_run_subprocess_capture_normalizes_input(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    runtime_subprocess = importlib.import_module("snail.runtime.subprocess")
    calls: dict[str, object] = {}

    class _Completed:
        stdout = "ok\n"

    def fake_run(cmd, **kwargs):
        calls["cmd"] = cmd
        calls["kwargs"] = kwargs
        return _Completed()

    monkeypatch.setattr(runtime_subprocess.subprocess, "run", fake_run)
    completed = runtime_subprocess._run_subprocess("echo hi", 123, capture=True)

    assert completed.stdout == "ok\n"
    assert calls["cmd"] == "echo hi"
    assert calls["kwargs"] == {
        "shell": True,
        "check": True,
        "text": True,
        "stdout": subprocess.PIPE,
        "input": "123",
    }


def test_runtime_run_subprocess_status_without_input(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    runtime_subprocess = importlib.import_module("snail.runtime.subprocess")
    calls: dict[str, object] = {}

    def fake_run(cmd, **kwargs):
        calls["cmd"] = cmd
        calls["kwargs"] = kwargs
        return object()

    monkeypatch.setattr(runtime_subprocess.subprocess, "run", fake_run)
    runtime_subprocess._run_subprocess("echo hi", capture=False)

    assert calls["cmd"] == "echo hi"
    assert calls["kwargs"] == {
        "shell": True,
        "check": True,
        "text": True,
    }


def test_subprocess_capture_error_fallback_reraises(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    runtime_subprocess = importlib.import_module("snail.runtime.subprocess")
    err = subprocess.CalledProcessError(2, "bad capture")

    def fake_run_subprocess(cmd: str, input_data=None, *, capture: bool):
        raise err

    monkeypatch.setattr(runtime_subprocess, "_run_subprocess", fake_run_subprocess)
    capture = runtime_subprocess.SubprocessCapture("bad capture")

    with pytest.raises(subprocess.CalledProcessError) as excinfo:
        capture("input")
    assert excinfo.value is err

    fallback = getattr(excinfo.value, "__fallback__", None)
    assert callable(fallback)
    with pytest.raises(subprocess.CalledProcessError) as fallback_exc:
        fallback()
    assert fallback_exc.value is err


def test_subprocess_status_error_fallback_returns_returncode(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    runtime_subprocess = importlib.import_module("snail.runtime.subprocess")
    err = subprocess.CalledProcessError(7, "bad status")

    def fake_run_subprocess(cmd: str, input_data=None, *, capture: bool):
        raise err

    monkeypatch.setattr(runtime_subprocess, "_run_subprocess", fake_run_subprocess)
    status = runtime_subprocess.SubprocessStatus("bad status")

    with pytest.raises(subprocess.CalledProcessError) as excinfo:
        status()
    fallback = getattr(excinfo.value, "__fallback__", None)
    assert callable(fallback)
    assert fallback() == 7


# --- Tests for byte strings ---


@pytest.mark.parametrize(
    ("script", "expected"),
    [
        pytest.param('x = b"hello"\nprint(x)', "b'hello'", id="basic"),
        pytest.param(
            'y = "world"\nx = b"hello {y}"\nprint(x)',
            "b'hello world'",
            id="interpolation",
        ),
        pytest.param(r'x = rb"\n"' + "\nprint(len(x))", "2", id="raw-rb"),
        pytest.param(
            'x = b"hello" + b" world"\nprint(x)',
            "b'hello world'",
            id="operations",
        ),
        pytest.param(r'x = br"\t"' + "\nprint(len(x))", "2", id="raw-br"),
    ],
)
def test_byte_string_variants(
    capsys: pytest.CaptureFixture[str], script: str, expected: str
) -> None:
    result, captured = run_cli(capsys, ["-P", script])
    assert result == 0
    assert captured.out.strip() == expected


def test_fstring_conversion_and_format_spec(capsys: pytest.CaptureFixture[str]) -> None:
    """Test f-string conversions and format specs."""
    script = 'value = "hi"\nprint("{value!r:>6}")'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.rstrip("\n") == "  'hi'"


def test_fstring_nested_format_spec(capsys: pytest.CaptureFixture[str]) -> None:
    """Test nested format spec interpolation."""
    script = 'value = 3.14159\nwidth = 6\nprec = 2\nprint("{value:{width}.{prec}f}")'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.rstrip("\n") == "  3.14"


def test_fstring_invalid_conversion_reports_syntax_error() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(['s = "{x!q}"'])
    assert "conversion" in str(excinfo.value)


def test_fstring_unterminated_expression_reports_syntax_error() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(["s = \"{'abc}\""])
    assert "unterminated f-string expression" in str(excinfo.value)


# --- Tests for example files ---

EXAMPLES_DIR = ROOT / "examples"


def test_example_all_syntax(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that examples/all_syntax.snail runs successfully."""
    result = main(["-f", str(EXAMPLES_DIR / "all_syntax.snail")])
    assert result == 0, f"all_syntax.snail failed with exit code {result}"
    captured = capsys.readouterr()
    # Verify some expected output to ensure the script actually ran
    assert "automatically printed" in captured.out


def test_example_json(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that examples/json.snail runs successfully."""
    result = main(["-P", "-f", str(EXAMPLES_DIR / "json.snail")])
    assert result == 0, f"json.snail failed with exit code {result}"


def test_example_awk(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Test that examples/awk.snail runs successfully."""
    monkeypatch.setattr(sys, "stdin", io.StringIO("demo line\nother line\n"))
    result = main(["--awk", "-f", str(EXAMPLES_DIR / "awk.snail")])
    assert result == 0, f"awk.snail failed with exit code {result}"
    captured = capsys.readouterr()
    # Verify expected output from the awk script
    assert "demo begin" in captured.out
    assert "demo end" in captured.out


def _snail_block_to_source(block: str) -> Optional[str]:
    lines = block.splitlines()
    if lines and lines[0].startswith("#!"):
        lines = lines[1:]
    source = "\n".join(lines).strip()
    if not source:
        return None
    return source


def _parse_snail_header(header: str) -> tuple[str, Optional[str]]:
    if header == "snail":
        return ("snail", None)
    if header == "snail-map":
        return ("snail-map", None)
    if header.startswith("snail-awk"):
        if header == "snail-awk":
            return ("snail-awk", None)
        if header.startswith("snail-awk(") and header.endswith(")"):
            raw = header[len("snail-awk(") : -1].strip()
            if not raw:
                return ("snail-awk", "")
            try:
                value = ast.literal_eval(raw)
            except (SyntaxError, ValueError) as exc:
                raise ValueError(f"invalid snail-awk stdin header: {header}") from exc
            if not isinstance(value, str):
                raise ValueError(f"snail-awk stdin must be a string: {header}")
            return ("snail-awk", value)
    raise ValueError(f"unsupported README fence: {header}")


def _collect_readme_snail_sources(
    path: Path,
) -> list[tuple[str, int, str, Optional[str]]]:
    content = path.read_text()
    sources: list[tuple[str, int, str, Optional[str]]] = []

    fence_re = re.compile(
        r"```(?P<header>snail(?:-awk(?:\([^)]*\))?|-map)?)\n(?P<body>.*?)\n```",
        re.S,
    )
    for match in fence_re.finditer(content):
        header = match.group("header")
        lang, stdin_input = _parse_snail_header(header)
        body = match.group("body")
        line_no = content.count("\n", 0, match.start()) + 1
        source = _snail_block_to_source(body)
        if source:
            sources.append((lang, line_no, source, stdin_input))
    return sources


_README_SNIPPETS = _collect_readme_snail_sources(ROOT / "README.md")
_README_SNIPPET_IDS = [
    f"{lang}@README.md:{line_no}" for lang, line_no, _, _ in _README_SNIPPETS
]


def _collect_readme_oneliners(path: Path) -> list[tuple[int, str, list[str]]]:
    content = path.read_text()
    oneliners: list[tuple[int, str, list[str]]] = []
    fence_re = re.compile(r"```bash\n(?P<body>.*?)\n```", re.S)
    for match in fence_re.finditer(content):
        body = match.group("body")
        start_line = content.count("\n", 0, match.start()) + 1
        for index, line in enumerate(body.splitlines()):
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            line_no = start_line + 1 + index
            try:
                mode, argv = _parse_oneliner_command(stripped)
                oneliners.append((line_no, mode, argv))
            except Exception:
                pass
    return oneliners


def _parse_oneliner_command(command: str) -> tuple[str, list[str]]:
    tokens = shlex.split(command)
    idx = tokens.index("snail")
    tokens = tokens[idx + 1 :]
    mode = "snail"
    i = 0
    while i < len(tokens):
        tok = tokens[i]
        if tok in ("-a", "--awk"):
            if mode != "snail":
                raise ValueError("oneliner cannot mix --awk and --map")
            mode = "awk"
            i += 1
            continue
        if tok in ("-m", "--map"):
            if mode != "snail":
                raise ValueError("oneliner cannot mix --awk and --map")
            mode = "map"
            i += 1
            continue
        if tok == "x=$my_bashvar":
            tok = "x=123"
        break
    argv = tokens[i:]
    if not argv:
        raise ValueError(f"oneliner missing code: {command}")
    return mode, argv


def _replace_map_oneliner_args(argv: list[str], map_file: Path) -> list[str]:
    idx = 0
    while idx < len(argv):
        tok = argv[idx]
        if tok in ("-b", "--begin", "-e", "--end", "-f"):
            idx += 2
            continue
        if tok.startswith("-"):
            idx += 1
            continue
        idx += 1
        break
    return [*argv[:idx], str(map_file)]


_README_ONELINERS = _collect_readme_oneliners(ROOT / "README.md")
if _README_ONELINERS:
    _README_ONELINER_PARAMS = [
        pytest.param(line_no, mode, argv, id=f"oneliner@README.md:{line_no}")
        for line_no, mode, argv in _README_ONELINERS
    ]
else:
    _README_ONELINER_PARAMS = [
        pytest.param(
            0,
            "snail",
            [],
            marks=pytest.mark.skip(
                reason="no ```snail-oneliner blocks found in README.md"
            ),
            id="no-oneliners",
        )
    ]


@pytest.mark.parametrize(
    "lang,line_no,source,stdin_input",
    _README_SNIPPETS,
    ids=_README_SNIPPET_IDS,
)
def test_readme_snail_blocks_parse(
    lang: str,
    line_no: int,
    source: str,
    stdin_input: Optional[str],
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    path = ROOT / "README.md"

    def _fake_run(cmd, shell=False, check=False, text=False, input=None, stdout=None):
        out = "" if text else b""
        return subprocess.CompletedProcess(cmd, 0, stdout=out)

    monkeypatch.setattr(subprocess, "run", _fake_run)
    if lang == "snail-awk":
        if stdin_input is not None:
            sys.stdin = io.StringIO(stdin_input)
        assert main(["--awk", source]) == 0, f"failed at {path}:{line_no}"
    elif lang == "snail-map":
        map_file = _ensure_readme_map_file(tmp_path)
        assert (
            main(["--map", source, str(map_file)]) == 0
        ), f"failed at {path}:{line_no}"
    else:
        combined = f"{README_SNIPPET_PREAMBLE}\n{source}"
        assert main([combined]) == 0, f"failed at {path}:{line_no}"


@pytest.mark.parametrize(
    "line_no,mode,argv",
    _README_ONELINER_PARAMS,
)
def test_readme_snail_oneliners(
    line_no: int,
    mode: str,
    argv: list[str],
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    path = ROOT / "README.md"

    def _fake_run(cmd, shell=False, check=False, text=False, input=None, stdout=None):
        out = "" if text else b""
        return subprocess.CompletedProcess(cmd, 0, stdout=out)

    monkeypatch.setattr(subprocess, "run", _fake_run)
    if mode == "awk":
        assert main(["--awk", *argv]) == 0, f"failed at {path}:{line_no}"
    elif mode == "map":
        map_file = _ensure_readme_map_file(tmp_path)
        map_argv = _replace_map_oneliner_args(argv, map_file)
        assert main(["--map", *map_argv]) == 0, f"failed at {path}:{line_no}"
    else:
        argv = ["-b", README_SNIPPET_PREAMBLE] + argv

        # Hackjobs for some test cases
        os.environ["my_bashvar"] = "123"
        try:
            # Special case hackjob since we don't actually run a shell
            if argv[3] == "x=$my_bashvar":
                argv[3] = "x=123"
        except IndexError:
            pass

        assert main(argv) == 0, f"failed at {path}:{line_no}"


# Map mode tests


def test_map_mode_from_args(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Test map mode with files passed as CLI arguments."""
    file_a = tmp_path / "a.txt"
    file_b = tmp_path / "b.txt"
    file_a.write_text("hello")
    file_b.write_text("world")
    result = main(["--map", "print($src)", str(file_a), str(file_b)])
    assert result == 0
    captured = capsys.readouterr()
    assert str(file_a) in captured.out
    assert str(file_b) in captured.out


def test_map_mode_dash_reads_stdin(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("stdin data"))
    result = main(["--map", "print($src)\nprint($text)", "-"])
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == ["-", "stdin data"]


def test_map_mode_missing_file_src_only(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    missing = tmp_path / "missing.txt"
    result = main(["--map", "print($src)", str(missing)])
    assert result == 0
    captured = capsys.readouterr()
    assert str(missing) in captured.out


def test_map_mode_missing_file_fd_access(tmp_path: Path) -> None:
    missing = tmp_path / "missing.txt"
    with pytest.raises(FileNotFoundError):
        main(["--map", "print($fd.read())", str(missing)])


def test_map_mode_missing_file_text_access(tmp_path: Path) -> None:
    missing = tmp_path / "missing.txt"
    with pytest.raises(FileNotFoundError):
        main(["--map", "print($text)", str(missing)])


def test_map_mode_text_content(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Test that $text contains file content."""
    file_a = tmp_path / "a.txt"
    file_a.write_text("hello world")
    result = main(["--map", "print(len($text))", str(file_a)])
    assert result == 0
    captured = capsys.readouterr()
    assert "11" in captured.out


def test_map_mode_fd_access(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Test that $fd is a readable file handle."""
    file_a = tmp_path / "a.txt"
    file_a.write_text("first line\nsecond line\n")
    result = main(["--map", "print($fd.readline().strip())", str(file_a)])
    assert result == 0
    captured = capsys.readouterr()
    assert "first line" in captured.out


def test_map_mode_lazy_text(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Test that $text is lazy (can use $fd first, then $text is empty)."""
    file_a = tmp_path / "a.txt"
    file_a.write_text("content")
    # Reading $fd first consumes the file, so $text will be empty
    result = main(["--map", "_ = $fd.read(); print(repr(str($text)))", str(file_a)])
    assert result == 0
    captured = capsys.readouterr()
    assert "''" in captured.out


def test_map_begin_end_flags(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    file_a = tmp_path / "a.txt"
    file_b = tmp_path / "b.txt"
    file_a.write_text("alpha")
    file_b.write_text("beta")
    result = main(
        [
            "--map",
            "-b",
            "print('start')",
            "-e",
            "print('done')",
            "print($src)",
            str(file_a),
            str(file_b),
        ]
    )
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == [
        "start",
        str(file_a),
        str(file_b),
        "done",
    ]


def test_map_multiple_begin_end_flags(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    file_a = tmp_path / "a.txt"
    file_a.write_text("alpha")
    result = main(
        [
            "--map",
            "--begin",
            "print('b1')",
            "-b",
            "print('b2')",
            "print($src)",
            "-e",
            "print('e1')",
            "--end",
            "print('e2')",
            str(file_a),
        ]
    )
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == [
        "b1",
        "b2",
        str(file_a),
        "e1",
        "e2",
    ]


def test_map_begin_end_oneliner_whitespace(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    file_a = tmp_path / "a.txt"
    file_a.write_text("alpha")
    result = main(["--map", "BEGIN {1} $src END {2}", str(file_a)])
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == ["1", str(file_a), "2"]


def test_map_begin_end_file_and_cli_order(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    map_file = tmp_path / "file1"
    map_file.write_text("readme map input\n")
    script = tmp_path / "script.snail"
    script.write_text(
        "BEGIN { print('file begin') }\nprint($src)\nEND { print('file end') }\n"
    )
    result = main(
        [
            "--map",
            "-b",
            "print('cli begin')",
            "-e",
            "print('cli end')",
            "-f",
            str(script),
            str(map_file),
        ]
    )
    assert result == 0
    captured = capsys.readouterr()
    assert captured.out.splitlines() == [
        "cli begin",
        "file begin",
        str(map_file),
        "file end",
        "cli end",
    ]


def test_map_begin_end_flags_reject_map_vars(tmp_path: Path) -> None:
    file_a = tmp_path / "a.txt"
    file_a.write_text("alpha")
    with pytest.raises(SyntaxError):
        main(
            [
                "--map",
                "-b",
                "print($src)",
                "print($src)",
                str(file_a),
            ]
        )


def test_map_identifiers_require_map_mode(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that $src is rejected in snail mode."""
    with pytest.raises(SyntaxError) as excinfo:
        main(["print($src)"])
    assert "map or awk mode" in str(excinfo.value)


def test_map_identifiers_require_map_mode_in_fstring_interpolation() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(['print("{$src}")'])
    assert "map or awk mode" in str(excinfo.value)


def test_map_identifiers_require_map_mode_in_subprocess_interpolation() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(["x = $(echo {$src})"])
    assert "map or awk mode" in str(excinfo.value)


def test_map_identifiers_require_map_mode_in_regex_interpolation() -> None:
    with pytest.raises(SyntaxError) as excinfo:
        main(['print("x" in /{$src}/)'])
    assert "map or awk mode" in str(excinfo.value)


def test_map_identifiers_require_map_mode_in_lambda_call_arguments() -> None:
    for source in [
        "f = def() { g($src) }",
        "f = def() { g(k=$src) }",
        "f = def() { g(*$src) }",
        "f = def() { g(**$src) }",
    ]:
        with pytest.raises(SyntaxError) as excinfo:
            main([source])
        assert "map or awk mode" in str(excinfo.value)


def test_map_begin_end_flags_reject_map_vars_fd_text(tmp_path: Path) -> None:
    file_a = tmp_path / "a.txt"
    file_a.write_text("alpha")
    for begin_snippet in ["print($fd)", "print($text)"]:
        with pytest.raises(SyntaxError):
            main(
                [
                    "--map",
                    "-b",
                    begin_snippet,
                    "print($src)",
                    str(file_a),
                ]
            )


def test_awk_and_map_mutually_exclusive(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that --awk and --map cannot be used together."""
    result = main(["--awk", "--map", "print('test')"])
    assert result == 2
    captured = capsys.readouterr()
    assert "--awk and --map cannot be used together" in captured.err


def test_example_map(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Test that examples/map.snail runs successfully."""
    file_a = tmp_path / "test.txt"
    file_a.write_text("test content here\n")
    result = main(["--map", "-f", str(EXAMPLES_DIR / "map.snail"), str(file_a)])
    assert result == 0, f"map.snail failed with exit code {result}"
    captured = capsys.readouterr()
    assert str(file_a) in captured.out
    assert "bytes" in captured.out
