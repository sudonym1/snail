from __future__ import annotations

import importlib.util
import io
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
