from __future__ import annotations

import importlib
import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
PYTHON_DIR = ROOT / "python"
sys.path.insert(0, str(PYTHON_DIR))

if importlib.util.find_spec("snail._native") is None:
    pytest.skip("snail extension not built", allow_module_level=True)

main = importlib.import_module("snail.cli").main

SEMANTIC_EQUIVALENCE_CASES = [
    pytest.param(
        "print(1 + 1)",
        "print(1\n+\n1)",
        id="infix-expression-newline-continuation",
    ),
    pytest.param(
        'call_value = print(1)\nparen_value = (1)\nlist_value = [1, 2]\ndict_value = %{"a": 1, "b": 2}\nsum_value = 1 + 2\nassigned = 3\nprint(call_value, paren_value, list_value, dict_value["b"], sum_value, assigned)',
        'call_value = print(\n1\n)\nparen_value = (\n1\n)\nlist_value = [1,\n2]\ndict_value = %{"a": 1,\n"b": 2}\nsum_value = 1 +\n2\nassigned =\n3\nprint(call_value, paren_value, list_value, dict_value["b"], sum_value, assigned)',
        id="mixed-expression-and-assignment-continuations",
    ),
    pytest.param(
        "def ret() { return 1 }\nprint(ret())",
        "def ret() { return\n1 }\nprint(ret())",
        id="return-newline-continuation",
    ),
    pytest.param(
        'err = ValueError("root")\ndef boom() { raise ValueError("bad") from err }\ntry { boom() }\nexcept ValueError as e { print(type(e.__cause__).__name__, e.args[0]) }',
        'err = ValueError("root")\ndef boom() { raise\nValueError("bad")\nfrom\nerr }\ntry { boom() }\nexcept ValueError\nas\ne\n{ print(type(e.__cause__).__name__, e.args[0]) }',
        id="raise-from-newline-continuation",
    ),
]


def run_program(
    capsys: pytest.CaptureFixture[str], source: str
) -> tuple[int, str, str]:
    exit_code = main(["-P", source])
    captured = capsys.readouterr()
    return exit_code, captured.out, captured.err


@pytest.mark.parametrize(("baseline", "variant"), SEMANTIC_EQUIVALENCE_CASES)
def test_whitespace_semantic_differential_equivalence(
    capsys: pytest.CaptureFixture[str], baseline: str, variant: str
) -> None:
    baseline_result = run_program(capsys, baseline)
    variant_result = run_program(capsys, variant)

    assert baseline_result == variant_result, (
        "whitespace semantic differential failed\n"
        f"baseline source:\n{baseline}\n"
        f"variant source:\n{variant}\n"
        f"baseline result: {baseline_result}\n"
        f"variant result: {variant_result}"
    )
