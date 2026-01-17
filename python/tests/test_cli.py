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

README_SNIPPET_PREAMBLE = """
def risky(*args, fail=False) { if fail { raise Exception(fail) } else { return args } }
def fetch_url(x) { return None }
def greet(*args, **kwargs) { print(*args) }
name = "world"
bad_email = "bad@@email"
phone = "867-5309"
"""

def test_parse_only(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--debug", "x = 1"]) == 0
    captured = capsys.readouterr()
    assert captured.out.strip() == "x = 1"


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
        r"```(?P<header>snail(?:-awk(?:\([^)]*\))?)?)\n(?P<body>.*?)\n```",
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


def _collect_readme_oneliners(path: Path) -> list[tuple[int, bool, list[str]]]:
    content = path.read_text()
    oneliners: list[tuple[int, bool, list[str]]] = []
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
                awk, argv = _parse_oneliner_command(stripped)
                oneliners.append((line_no, awk, argv))
            except Exception:
                pass
    return oneliners


def _parse_oneliner_command(command: str) -> tuple[bool, list[str]]:
    tokens = shlex.split(command)
    idx = tokens.index("snail")
    tokens = tokens[idx+1:]
    awk = False
    i = 0
    while i < len(tokens):
        tok = tokens[i]
        if tok in ("-a", "--awk"):
            awk = True
            i += 1
            continue
        break
    argv = tokens[i:]
    if not argv:
        raise ValueError(f"oneliner missing code: {command}")
    return awk, argv


_README_ONELINERS = _collect_readme_oneliners(ROOT / "README.md")
if _README_ONELINERS:
    _README_ONELINER_PARAMS = [
        pytest.param(line_no, awk, argv, id=f"oneliner@README.md:{line_no}")
        for line_no, awk, argv in _README_ONELINERS
    ]
else:
    _README_ONELINER_PARAMS = [
        pytest.param(
            0,
            False,
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
    lang: str, line_no: int, source: str, stdin_input: str | None, monkeypatch: pytest.MonkeyPatch
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
    else:
        combined = f"{README_SNIPPET_PREAMBLE}\n{source}"
        assert main([combined]) == 0, f"failed at {path}:{line_no}"


@pytest.mark.parametrize(
    "line_no,awk,argv",
    _README_ONELINER_PARAMS,
)
def test_readme_snail_oneliners(
    line_no: int, awk: bool, argv: list[str], monkeypatch: pytest.MonkeyPatch
) -> None:
    path = ROOT / "README.md"

    def _fake_run(cmd, shell=False, check=False, text=False, input=None, stdout=None):
        out = "" if text else b""
        return subprocess.CompletedProcess(cmd, 0, stdout=out)

    monkeypatch.setattr(subprocess, "run", _fake_run)
    if awk:
        assert main(["--awk", *argv]) == 0, f"failed at {path}:{line_no}"
    else:
        combined = f"{README_SNIPPET_PREAMBLE}\n{argv[0]}"
        assert main([combined, *argv[1:]]) == 0, f"failed at {path}:{line_no}"
