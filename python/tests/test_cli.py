from __future__ import annotations

import importlib.util
import io
import re
import shlex
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
PYTHON_DIR = ROOT / "python"
sys.path.insert(0, str(PYTHON_DIR))

if importlib.util.find_spec("snail._native") is None:
    pytest.skip("snail extension not built", allow_module_level=True)

from snail.cli import main


def test_parse_only() -> None:
    assert main(["--parse-only", "x = 1"]) == 0


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


def test_awk_mode(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("foo\nbar\n"))
    assert main(["--awk", "/foo/ { print($l) }"]) == 0
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
        main(["print($l)"])
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


def _extract_snail_commands(line: str) -> str | None:
    line = line.strip()
    if not line or line.startswith("#"):
        return None
    if "| snail" in line:
        line = line.split("| snail", 1)[1].strip()
        if not line.startswith("snail"):
            line = f"snail {line}"
    if line.startswith("uv run -- snail "):
        line = line[len("uv run -- ") :]
    if not line.startswith("snail "):
        return None
    return line


def _argv_from_snail_command(command: str) -> list[str] | None:
    tokens = shlex.split(command)
    if not tokens or tokens[0] != "snail":
        return None
    tokens = tokens[1:]
    argv: list[str] = ["--parse-only"]
    awk = False
    file_path: str | None = None
    code: str | None = None

    i = 0
    while i < len(tokens):
        tok = tokens[i]
        if tok in ("-a", "--awk"):
            awk = True
            i += 1
            continue
        if tok == "-f" and i + 1 < len(tokens):
            file_path = tokens[i + 1]
            i += 2
            continue
        if tok.startswith("-"):
            i += 1
            continue
        code = tok
        break

    if awk:
        argv.append("--awk")
    if file_path:
        path = Path(file_path)
        if not path.is_absolute():
            path = ROOT / path
        argv.extend(["-f", str(path)])
        return argv
    if code is None:
        return None
    argv.append(code)
    return argv


def _snail_block_to_argv(block: str) -> list[str] | None:
    lines = block.splitlines()
    mode = "snail"
    if lines and lines[0].startswith("#!"):
        if "--awk" in lines[0]:
            mode = "awk"
        lines = lines[1:]
    source = "\n".join(lines).strip()
    if not source:
        return None
    argv = ["--parse-only"]
    if mode == "awk":
        argv.append("--awk")
    argv.append(source)
    return argv


def _collect_doc_snail_argvs(path: Path) -> list[list[str]]:
    content = path.read_text()
    argvs: list[list[str]] = []

    fence_re = re.compile(r"```(?P<lang>[A-Za-z0-9_-]*)\n(?P<body>.*?)\n```", re.S)
    for match in fence_re.finditer(content):
        lang = match.group("lang").lower()
        body = match.group("body")
        if lang == "snail":
            argv = _snail_block_to_argv(body)
            if argv:
                argvs.append(argv)
        elif lang in ("bash", "sh", "shell"):
            for line in body.splitlines():
                command = _extract_snail_commands(line)
                if not command:
                    continue
                argv = _argv_from_snail_command(command)
                if argv:
                    argvs.append(argv)

    inline_re = re.compile(r"`([^`]+)`")
    for snippet in inline_re.findall(content):
        command = _extract_snail_commands(snippet)
        if not command:
            continue
        argv = _argv_from_snail_command(command)
        if argv:
            argvs.append(argv)

    return argvs


@pytest.mark.parametrize("path", [ROOT / "README.md", ROOT / "AGENTS.md"])
def test_doc_snail_snippets_parse(path: Path) -> None:
    argvs = _collect_doc_snail_argvs(path)
    assert argvs, f"no snail snippets found in {path}"
    for argv in argvs:
        assert main(argv) == 0
