from __future__ import annotations

import importlib.util
import io
import ast
import re
import shlex
import subprocess
import sys
import traceback
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
PYTHON_DIR = ROOT / "python"
sys.path.insert(0, str(PYTHON_DIR))

if importlib.util.find_spec("snail._native") is None:
    pytest.skip("snail extension not built", allow_module_level=True)

from snail.cli import main
import snail

README_SNIPPET_PREAMBLE = """
def risky(*args, fail=False) { if fail { raise Exception(fail) } else { return args } }
def fetch_url(x) { return None }
def greet(*args, **kwargs) { print(*args) }
name = "world"
bad_email = "bad@@email"
phone = "867-5309"
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


def test_debug_snail_ast_file(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    script = tmp_path / "script.snail"
    script.write_text("x = 1")
    assert main(["--debug-snail-ast", "-f", str(script)]) == 0
    captured = capsys.readouterr()
    assert "Program" in captured.out


def test_debug_snail_ast_reports_parse_error(capsys: pytest.CaptureFixture[str]) -> None:
    with pytest.raises(SyntaxError):
        main(["--debug-snail-ast", "x ="])


def test_parse_ast_api_basic() -> None:
    result = snail.parse_ast("x = 1")
    assert "Program" in result
    assert "Assign" in result


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


def test_traceback_highlights_inline_snail() -> None:
    with pytest.raises(NameError) as excinfo:
        main(["x"])
    filenames = [
        frame.filename
        for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert "snail:<cmd>" in filenames


def test_traceback_highlights_file_snail(tmp_path: Path) -> None:
    script = tmp_path / "script.snail"
    script.write_text("x\n")
    with pytest.raises(NameError) as excinfo:
        main(["-f", str(script)])
    filenames = [
        frame.filename
        for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert f"snail:{script}" in filenames


def test_traceback_highlights_library_snail() -> None:
    import snail

    with pytest.raises(NameError) as excinfo:
        snail.exec("x", filename="lib.snail")
    filenames = [
        frame.filename
        for frame in traceback.extract_tb(excinfo.value.__traceback__)
    ]
    assert "snail:lib.snail" in filenames


@pytest.fixture(autouse=True)
def _stdin_devnull(monkeypatch: pytest.MonkeyPatch) -> None:
    with open("/dev/null", "r") as handle:
        monkeypatch.setattr(sys, "stdin", handle)
        yield


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
            "if let [user, domain] = pair { print(domain) } else { print(\"no\") }",
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


def test_combined_short_flags_awk(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo\n"))
    assert main(["-aP", "/foo/ { print($0) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "foo\n"


def test_combined_short_flag_with_value(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = tmp_path / "script.snail"
    script.write_text("/foo/ { print($0) }\n")
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo\nbar\n"))
    assert main(["-af", str(script)]) == 0
    captured = capsys.readouterr()
    assert captured.out == "foo\n"


def test_combined_short_flag_with_attached_value(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    script = tmp_path / "script.snail"
    script.write_text("/foo/ { print($0) }\n")
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo\nbar\n"))
    assert main([f"-af{script}"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "foo\n"


def test_combined_short_help(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["-ah"]) == 0
    captured = capsys.readouterr()
    assert "usage:" in captured.out


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


def test_awk_mode(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo\nbar\n"))
    assert main(["--awk", "/foo/ { print($0) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "foo\n"


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


def test_awk_begin_flag(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("line\n"))
    assert main(["--awk", "-b", "print('start')", "{ print($0) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "start\nline\n"


def test_awk_begin_long_flag(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("line\n"))
    assert main(["--awk", "--begin", "print('start')", "{ print($0) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "start\nline\n"


def test_awk_end_flag(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("line\n"))
    assert main(["--awk", "-e", "print('done')", "{ print($0) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "line\ndone\n"


def test_awk_end_long_flag(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("line\n"))
    assert main(["--awk", "--end", "print('done')", "{ print($0) }"]) == 0
    captured = capsys.readouterr()
    assert captured.out == "line\ndone\n"


def test_awk_multiple_begin_end_flags(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("x\n"))
    assert main([
        "--awk",
        "-b", "print('b1')",
        "--begin", "print('b2')",
        "-e", "print('e1')",
        "--end", "print('e2')",
        "{ print($0) }",
    ]) == 0
    captured = capsys.readouterr()
    assert captured.out == "b1\nb2\nx\ne1\ne2\n"


def test_awk_begin_end_interleaved_order(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("x\n"))
    # -b before code, -e after code
    assert main([
        "--awk",
        "-b", "print('start')",
        "{ print($0) }",
        "-e", "print('end')",
    ]) == 0
    captured = capsys.readouterr()
    assert captured.out == "start\nx\nend\n"


def test_awk_begin_after_args(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
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


def test_begin_end_without_mode_fails(capsys: pytest.CaptureFixture[str]) -> None:
    result = main(["--begin", "print('x')", "print('y')"])
    assert result == 2
    captured = capsys.readouterr()
    assert "-b/--begin and -e/--end options require --awk or --map mode" in captured.err


# --- Tests for auto-import ---


def test_auto_import_sys(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that sys is available without explicit import."""
    assert main(["-P", "print(sys.version_info.major)"]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip().isdigit()


def test_auto_import_os(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that os is available without explicit import."""
    assert main(["-P", "print(os.name)"]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() in ("posix", "nt")


def test_no_auto_import_flag(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that --no-auto-import disables auto-import."""
    with pytest.raises(NameError) as excinfo:
        main(["--no-auto-import", "print(sys.version)"])
    assert "sys" in str(excinfo.value)


def test_no_auto_import_short_flag(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that -I disables auto-import."""
    with pytest.raises(NameError) as excinfo:
        main(["-I", "print(os.name)"])
    assert "os" in str(excinfo.value)


def test_auto_import_user_shadow(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that user can shadow auto-imported names."""
    assert main(["-P", 'sys = "custom"\nprint(sys)']) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "custom"


def test_auto_import_path(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that Path is available without explicit import."""
    assert main(["-P", "print(Path('.').resolve())"]) == 0
    captured = capsys.readouterr()
    # Should print an absolute path
    assert captured.out.strip().startswith("/")


def test_no_auto_import_path(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that -I disables Path auto-import."""
    with pytest.raises(NameError) as excinfo:
        main(["-I", "print(Path('.'))"])
    assert "Path" in str(excinfo.value)


# --- Tests for byte strings ---


def test_byte_string_basic(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that byte strings are parsed and executed correctly."""
    script = 'x = b"hello"\nprint(x)'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "b'hello'"


def test_byte_string_interpolation(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that byte strings support interpolation (unlike Python)."""
    script = 'y = "world"\nx = b"hello {y}"\nprint(x)'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    # Should interpolate and encode
    assert captured.out.strip() == "b'hello world'"


def test_raw_byte_string(capsys: pytest.CaptureFixture[str]) -> None:
    """Test raw byte strings."""
    script = r'x = rb"\n"' + '\nprint(len(x))'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    # rb"\n" should be 2 bytes: backslash and n
    assert captured.out.strip() == "2"


def test_byte_string_operations(capsys: pytest.CaptureFixture[str]) -> None:
    """Test byte string operations work correctly."""
    script = 'x = b"hello" + b" world"\nprint(x)'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "b'hello world'"


def test_byte_string_br_prefix(capsys: pytest.CaptureFixture[str]) -> None:
    """Test br prefix for raw byte strings."""
    script = r'x = br"\t"' + '\nprint(len(x))'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    # br"\t" should be 2 bytes: backslash and t
    assert captured.out.strip() == "2"


def test_fstring_conversion_and_format_spec(capsys: pytest.CaptureFixture[str]) -> None:
    """Test f-string conversions and format specs."""
    script = 'value = "hi"\nprint("{value!r:>6}")'
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.rstrip("\n") == "  'hi'"


def test_fstring_nested_format_spec(capsys: pytest.CaptureFixture[str]) -> None:
    """Test nested format spec interpolation."""
    script = "value = 3.14159\nwidth = 6\nprec = 2\nprint(\"{value:{width}.{prec}f}\")"
    assert main(["-P", script]) == 0
    captured = capsys.readouterr()
    assert captured.out.rstrip("\n") == "  3.14"


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


def _snail_block_to_source(block: str) -> str | None:
    lines = block.splitlines()
    if lines and lines[0].startswith("#!"):
        lines = lines[1:]
    source = "\n".join(lines).strip()
    if not source:
        return None
    return source


def _parse_snail_header(header: str) -> tuple[str, str | None]:
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


def _collect_readme_snail_sources(path: Path) -> list[tuple[str, int, str, str | None]]:
    content = path.read_text()
    sources: list[tuple[str, int, str, str | None]] = []

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
    tokens = tokens[idx+1:]
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
    stdin_input: str | None,
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
        assert main(["--map", source, str(map_file)]) == 0, f"failed at {path}:{line_no}"
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
        combined = f"{README_SNIPPET_PREAMBLE}\n{argv[0]}"
        assert main([combined, *argv[1:]]) == 0, f"failed at {path}:{line_no}"


# Map mode tests


def test_map_mode_from_args(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
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


def test_map_mode_fd_access(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Test that $fd is a readable file handle."""
    file_a = tmp_path / "a.txt"
    file_a.write_text("first line\nsecond line\n")
    result = main(["--map", "print($fd.readline().strip())", str(file_a)])
    assert result == 0
    captured = capsys.readouterr()
    assert "first line" in captured.out


def test_map_mode_lazy_text(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
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
    result = main([
        "--map",
        "-b",
        "print('start')",
        "-e",
        "print('done')",
        "print($src)",
        str(file_a),
        str(file_b),
    ])
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
    result = main([
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
    ])
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
    assert "map variables are only valid in map mode" in str(excinfo.value)


def test_awk_and_map_mutually_exclusive(capsys: pytest.CaptureFixture[str]) -> None:
    """Test that --awk and --map cannot be used together."""
    result = main(["--awk", "--map", "print('test')"])
    assert result == 2
    captured = capsys.readouterr()
    assert "--awk and --map cannot be used together" in captured.err


def test_example_map(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Test that examples/map.snail runs successfully."""
    file_a = tmp_path / "test.txt"
    file_a.write_text("test content here\n")
    result = main(["--map", "-f", str(EXAMPLES_DIR / "map.snail"), str(file_a)])
    assert result == 0, f"map.snail failed with exit code {result}"
    captured = capsys.readouterr()
    assert str(file_a) in captured.out
    assert "bytes" in captured.out
