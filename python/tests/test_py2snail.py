"""Tests for the Python-to-Snail translator."""

from __future__ import annotations

import importlib
import importlib.util
import io
import sys
from contextlib import redirect_stdout
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
PYTHON_DIR = ROOT / "python"
sys.path.insert(0, str(PYTHON_DIR))

if importlib.util.find_spec("snail._native") is None:
    pytest.skip("snail extension not built", allow_module_level=True)

snail = importlib.import_module("snail")
from snail.py2snail import Py2SnailError, translate  # noqa: E402

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _run_python(source: str) -> str:
    """Execute Python source and capture stdout."""
    buf = io.StringIO()
    code = compile(source, "<test>", "exec")
    with redirect_stdout(buf):
        exec(code, {"__name__": "__main__"})
    return buf.getvalue()


def _run_snail(source: str) -> str:
    """Execute Snail source and capture stdout."""
    buf = io.StringIO()
    old = sys.stdout
    sys.stdout = buf
    try:
        snail.exec(source, auto_print=False)
    finally:
        sys.stdout = old
    return buf.getvalue()


def _roundtrip(python_source: str) -> None:
    """Verify Python and its Snail translation produce identical stdout."""
    expected = _run_python(python_source)
    snail_source = translate(python_source)
    actual = _run_snail(snail_source)
    assert actual == expected, (
        f"Output mismatch.\n"
        f"Python output: {expected!r}\n"
        f"Snail source:\n{snail_source}\n"
        f"Snail output: {actual!r}"
    )


# ---------------------------------------------------------------------------
# Unit tests: individual transformation rules
# ---------------------------------------------------------------------------


class TestBlockSyntax:
    def test_if_else(self) -> None:
        result = translate("if True:\n    pass\nelse:\n    pass\n")
        assert "if True {" in result
        assert "} else {" not in result  # we emit else on its own line
        assert "else" in result

    def test_elif(self) -> None:
        result = translate("if x:\n    pass\nelif y:\n    pass\nelse:\n    pass\n")
        assert "elif y {" in result

    def test_for_loop(self) -> None:
        result = translate("for x in range(10):\n    pass\n")
        assert "for x in range(10) {" in result

    def test_while_loop(self) -> None:
        result = translate("while True:\n    break\n")
        assert "while True {" in result

    def test_function_def(self) -> None:
        result = translate("def foo(x, y):\n    return x + y\n")
        assert "def foo(x, y) {" in result
        assert "return x + y" in result

    def test_class_def(self) -> None:
        result = translate("class Foo:\n    pass\n")
        assert "class Foo {" in result

    def test_class_with_bases(self) -> None:
        result = translate("class Foo(Bar):\n    pass\n")
        assert "class Foo(Bar) {" in result

    def test_class_with_multiple_bases(self) -> None:
        result = translate("class Foo(Bar, Baz):\n    pass\n")
        assert "class Foo(Bar, Baz) {" in result

    def test_try_except(self) -> None:
        result = translate(
            "try:\n    pass\nexcept ValueError as e:\n    pass\n"
        )
        assert "try {" in result
        assert "except ValueError as e {" in result

    def test_try_finally(self) -> None:
        result = translate("try:\n    pass\nfinally:\n    pass\n")
        assert "try {" in result
        assert "finally {" in result

    def test_with_statement(self) -> None:
        result = translate("with open('f') as fh:\n    pass\n")
        assert 'with open("f") as fh {' in result


class TestCollectionLiterals:
    def test_dict_literal(self) -> None:
        result = translate('x = {"a": 1, "b": 2}\n')
        assert '%{"a": 1, "b": 2}' in result

    def test_empty_dict(self) -> None:
        result = translate("x = {}\n")
        assert "%{}" in result

    def test_set_literal(self) -> None:
        result = translate("x = {1, 2, 3}\n")
        assert "#{1, 2, 3}" in result

    def test_list_literal(self) -> None:
        result = translate("x = [1, 2, 3]\n")
        assert "[1, 2, 3]" in result

    def test_tuple_literal(self) -> None:
        result = translate("x = (1, 2)\n")
        assert "(1, 2)" in result

    def test_single_element_tuple(self) -> None:
        result = translate("x = (1,)\n")
        assert "(1,)" in result

    def test_empty_tuple(self) -> None:
        result = translate("x = ()\n")
        assert "()" in result


class TestComprehensions:
    def test_list_comp(self) -> None:
        result = translate("x = [i for i in range(5)]\n")
        assert "[i for i in range(5)]" in result

    def test_dict_comp(self) -> None:
        result = translate("x = {k: v for k, v in items}\n")
        assert "%{k: v for (k, v) in items}" in result or "%{k: v for k, v in items}" in result

    def test_set_comp(self) -> None:
        result = translate("x = {i for i in range(5)}\n")
        assert "#{" in result
        assert "for i in range(5)" in result

    def test_generator_exp(self) -> None:
        result = translate("x = sum(i for i in range(5))\n")
        assert "for i in range(5)" in result

    def test_comp_with_filter(self) -> None:
        result = translate("x = [i for i in range(10) if i > 3]\n")
        assert "if i > 3" in result


class TestStringHandling:
    def test_plain_string(self) -> None:
        result = translate('x = "hello"\n')
        assert '"hello"' in result

    def test_string_with_braces_escaped(self) -> None:
        result = translate('x = "a {b} c"\n')
        assert '"a {{b}} c"' in result

    def test_fstring_no_escape(self) -> None:
        result = translate('x = f"hello {name}"\n')
        assert '"hello {name}"' in result
        assert "f" not in result.split('"')[0]  # no 'f' prefix

    def test_fstring_with_format_spec(self) -> None:
        result = translate('x = f"{value:.2f}"\n')
        assert "{value:.2f}" in result

    def test_fstring_with_conversion(self) -> None:
        result = translate('x = f"{value!r}"\n')
        assert "{value!r}" in result


class TestLambda:
    def test_simple_lambda(self) -> None:
        result = translate("f = lambda x: x * 2\n")
        assert "def(x) { x * 2 }" in result

    def test_lambda_no_args(self) -> None:
        result = translate("f = lambda: 42\n")
        assert "def() { 42 }" in result

    def test_lambda_default_arg(self) -> None:
        result = translate("f = lambda x, y=1: x + y\n")
        assert "def(x, y=1)" in result


class TestIfExpression:
    def test_ternary(self) -> None:
        result = translate("x = a if cond else b\n")
        assert "if cond { a } else { b }" in result


class TestAnnotations:
    def test_annotated_assign_stripped(self) -> None:
        result = translate("x: int = 5\n")
        assert "x = 5" in result
        assert "int" not in result

    def test_annotation_only_skipped(self) -> None:
        result = translate("x: int\n")
        assert result.strip() == ""

    def test_function_annotations_stripped(self) -> None:
        result = translate("def f(x: int, y: str = 'a') -> bool:\n    pass\n")
        assert "int" not in result
        assert "str" not in result
        assert "bool" not in result
        assert "def f(x, y=" in result


class TestOperators:
    def test_augmented_assign(self) -> None:
        result = translate("x += 1\n")
        assert "x += 1" in result

    def test_boolean_ops(self) -> None:
        result = translate("x = a and b or c\n")
        assert "and" in result
        assert "or" in result

    def test_comparison_chain(self) -> None:
        result = translate("x = 1 < y <= 10\n")
        assert "1 < y" in result
        assert "<= 10" in result

    def test_unary_not(self) -> None:
        result = translate("x = not True\n")
        assert "not True" in result


class TestMiscStatements:
    def test_import(self) -> None:
        result = translate("import os\n")
        assert "import os" in result

    def test_from_import(self) -> None:
        result = translate("from os.path import join\n")
        assert "from os.path import join" in result

    def test_multi_import(self) -> None:
        result = translate("import os, sys\n")
        assert "import os, sys" in result

    def test_import_as(self) -> None:
        result = translate("import numpy as np\n")
        assert "import numpy as np" in result

    def test_assert_stmt(self) -> None:
        result = translate('assert x > 0, "must be positive"\n')
        assert "assert x > 0" in result

    def test_delete(self) -> None:
        result = translate("del x\n")
        assert "del x" in result

    def test_raise(self) -> None:
        result = translate('raise ValueError("bad")\n')
        assert 'raise ValueError("bad")' in result

    def test_raise_from(self) -> None:
        result = translate('raise ValueError("bad") from err\n')
        assert "from err" in result

    def test_global(self) -> None:
        with pytest.raises(Py2SnailError, match="global"):
            translate("global x, y\n")

    def test_nonlocal(self) -> None:
        with pytest.raises(Py2SnailError, match="nonlocal"):
            translate("def f():\n    x = 1\n    def g():\n        nonlocal x\n")

    def test_yield(self) -> None:
        result = translate("def f():\n    yield 1\n")
        assert "yield 1" in result

    def test_yield_from(self) -> None:
        result = translate("def f():\n    yield from [1, 2]\n")
        assert "yield from [1, 2]" in result

    def test_pass(self) -> None:
        result = translate("pass\n")
        assert "pass" in result

    def test_break(self) -> None:
        result = translate("while True:\n    break\n")
        assert "break" in result

    def test_continue(self) -> None:
        result = translate("for i in x:\n    continue\n")
        assert "continue" in result

    def test_return_none(self) -> None:
        result = translate("def f():\n    return\n")
        assert "return\n" in result

    def test_multi_target_assign(self) -> None:
        result = translate("a = b = 5\n")
        # Should desugar to two assignments
        assert "b = 5" in result
        assert "a = b" in result


class TestDecorators:
    def test_function_decorator_unsupported(self) -> None:
        with pytest.raises(Py2SnailError, match="decorator"):
            translate("@staticmethod\ndef f():\n    pass\n")

    def test_class_decorator_unsupported(self) -> None:
        with pytest.raises(Py2SnailError, match="decorator"):
            translate("@decorator\nclass Foo:\n    pass\n")


class TestStarArgs:
    def test_args_kwargs(self) -> None:
        result = translate("def f(*args, **kwargs):\n    pass\n")
        assert "*args" in result
        assert "**kwargs" in result

    def test_kwonly(self) -> None:
        result = translate("def f(a, *, b):\n    pass\n")
        assert "def f(a, *, b)" in result

    def test_starred_in_call(self) -> None:
        result = translate("f(*args, **kwargs)\n")
        assert "*args" in result
        assert "**kwargs" in result


class TestDictUnpacking:
    def test_dict_unpack(self) -> None:
        result = translate("x = {**a, **b}\n")
        assert "%{**a, **b}" in result


class TestSlicing:
    def test_simple_slice(self) -> None:
        result = translate("x = a[1:3]\n")
        assert "a[1:3]" in result

    def test_step_slice(self) -> None:
        # Snail doesn't support step slicing, so we use slice() builtin
        result = translate("x = a[::2]\n")
        assert "slice(None, None, 2)" in result


class TestForElse:
    def test_for_else(self) -> None:
        result = translate("for x in y:\n    pass\nelse:\n    pass\n")
        assert "for x in y {" in result
        assert "else" in result


# ---------------------------------------------------------------------------
# Unsupported constructs
# ---------------------------------------------------------------------------


class TestWalrusOperator:
    def test_walrus_translation(self) -> None:
        result = translate("if (x := 10) > 5:\n    pass\n")
        assert "{ x = 10; x }" in result

    def test_walrus_simple(self) -> None:
        result = translate("y = (x := 5)\n")
        assert "{ x = 5; x }" in result

    def test_walrus_roundtrip(self) -> None:
        _roundtrip(
            "if (n := 10) > 5:\n"
            "    print('big', n)\n"
        )

    def test_walrus_in_assignment_roundtrip(self) -> None:
        _roundtrip(
            "values = [1, 2, 3]\n"
            "total = (n := len(values)) * 10\n"
            "print(total, n)\n"
        )


class TestUnsupported:
    def test_async_function(self) -> None:
        with pytest.raises(Py2SnailError, match="async"):
            translate("async def f():\n    pass\n")

    def test_await(self) -> None:
        # async def is hit first, so we get "async" error
        with pytest.raises(Py2SnailError, match="async"):
            translate("async def f():\n    await g()\n")

    def test_async_for(self) -> None:
        # async def is hit first
        with pytest.raises(Py2SnailError, match="async"):
            translate("async def f():\n    async for x in y:\n        pass\n")

    def test_async_with(self) -> None:
        # async def is hit first
        with pytest.raises(Py2SnailError, match="async"):
            translate("async def f():\n    async with x as y:\n        pass\n")

    def test_matmul_operator(self) -> None:
        with pytest.raises(Py2SnailError, match="binary operator"):
            translate("x = a @ b\n")

    def test_matmul_augassign(self) -> None:
        with pytest.raises(Py2SnailError, match="augmented assignment"):
            translate("a @= b\n")

    def test_positional_only_params(self) -> None:
        with pytest.raises(Py2SnailError, match="positional-only"):
            translate("def f(a, /, b):\n    pass\n")

    @pytest.mark.skipif(sys.version_info < (3, 12), reason="type statement requires 3.12+")
    def test_type_alias(self) -> None:
        with pytest.raises(Py2SnailError, match="type alias"):
            translate("type Point = tuple[int, int]\n")

    def test_dict_unpacking_in_literal(self) -> None:
        result = translate("x = {**a, 'b': 2}\n")
        assert "**a" in result


# ---------------------------------------------------------------------------
# Round-trip tests: run Python, translate, run Snail, compare output
# ---------------------------------------------------------------------------


class TestRoundTrip:
    def test_arithmetic(self) -> None:
        _roundtrip("print(1 + 2 * 3)\nprint(10 // 3)\nprint(2 ** 8)\n")

    def test_string_ops(self) -> None:
        _roundtrip('print("hello" + " " + "world")\nprint("ha" * 3)\n')

    def test_variables(self) -> None:
        _roundtrip("x = 42\ny = x + 8\nprint(x, y)\n")

    def test_if_else(self) -> None:
        _roundtrip(
            "x = 10\nif x > 5:\n    print('big')\nelse:\n    print('small')\n"
        )

    def test_elif(self) -> None:
        _roundtrip(
            "x = 5\nif x > 10:\n    print('a')\n"
            "elif x > 3:\n    print('b')\n"
            "else:\n    print('c')\n"
        )

    def test_for_loop(self) -> None:
        _roundtrip("for i in range(5):\n    print(i)\n")

    def test_while_loop(self) -> None:
        _roundtrip("x = 3\nwhile x > 0:\n    print(x)\n    x -= 1\n")

    def test_function(self) -> None:
        _roundtrip(
            "def add(a, b):\n    return a + b\nprint(add(3, 4))\n"
        )

    def test_function_default_args(self) -> None:
        _roundtrip(
            "def greet(name, greeting='hello'):\n"
            "    return greeting + ' ' + name\n"
            "print(greet('world'))\n"
            "print(greet('world', 'hi'))\n"
        )

    def test_class(self) -> None:
        # Note: avoid augmented assignment on attributes inside classes
        # (self.x += 1) due to Snail runtime name-mangling issue
        _roundtrip(
            "class Counter:\n"
            "    def __init__(self, start):\n"
            "        self.count = start\n"
            "    def inc(self):\n"
            "        self.count = self.count + 1\n"
            "        return self.count\n"
            "c = Counter(0)\n"
            "print(c.inc())\n"
            "print(c.inc())\n"
        )

    def test_list_operations(self) -> None:
        _roundtrip(
            "xs = [1, 2, 3]\nxs.append(4)\nprint(xs)\nprint(xs[0])\nprint(xs[-1])\n"
        )

    def test_dict_operations(self) -> None:
        _roundtrip(
            "d = {'a': 1, 'b': 2}\n"
            "d['c'] = 3\n"
            "print(sorted(d.keys()))\n"
            "print(d['a'])\n"
        )

    def test_set_operations(self) -> None:
        _roundtrip(
            "s = {1, 2, 3}\nprint(1 in s)\nprint(4 in s)\nprint(sorted(s))\n"
        )

    def test_list_comprehension(self) -> None:
        _roundtrip("print([x * 2 for x in range(5)])\n")

    def test_dict_comprehension(self) -> None:
        _roundtrip("print(sorted({k: k*2 for k in range(3)}.items()))\n")

    def test_nested_if(self) -> None:
        _roundtrip(
            "for i in range(4):\n"
            "    if i % 2 == 0:\n"
            "        print(i, 'even')\n"
            "    else:\n"
            "        print(i, 'odd')\n"
        )

    def test_try_except(self) -> None:
        _roundtrip(
            "try:\n"
            "    x = 1 / 0\n"
            "except ZeroDivisionError:\n"
            "    print('caught')\n"
        )

    def test_try_except_as(self) -> None:
        _roundtrip(
            "try:\n"
            "    int('bad')\n"
            "except ValueError as e:\n"
            "    print('error:', e)\n"
        )

    def test_try_else_finally(self) -> None:
        _roundtrip(
            "try:\n"
            "    x = 1\n"
            "except:\n"
            "    print('fail')\n"
            "else:\n"
            "    print('ok')\n"
            "finally:\n"
            "    print('done')\n"
        )

    def test_string_interpolation(self) -> None:
        _roundtrip(
            "name = 'world'\nprint(f'hello {name}')\n"
        )

    def test_fstring_format_spec(self) -> None:
        _roundtrip("x = 3.14159\nprint(f'{x:.2f}')\n")

    def test_multiline_function(self) -> None:
        _roundtrip(
            "def fib(n):\n"
            "    if n <= 1:\n"
            "        return n\n"
            "    return fib(n - 1) + fib(n - 2)\n"
            "print(fib(10))\n"
        )

    def test_star_args(self) -> None:
        _roundtrip(
            "def f(*args, **kwargs):\n"
            "    print(args, sorted(kwargs.items()))\n"
            "f(1, 2, a=3)\n"
        )

    def test_tuple_unpacking(self) -> None:
        _roundtrip(
            "a, b = 1, 2\nprint(a, b)\nfirst, *rest = [1, 2, 3]\nprint(first, rest)\n"
        )

    def test_lambda(self) -> None:
        _roundtrip(
            "double = lambda x: x * 2\nprint(double(5))\n"
        )

    def test_ternary(self) -> None:
        _roundtrip(
            "x = 10\nprint('big' if x > 5 else 'small')\n"
        )

    def test_nested_collections(self) -> None:
        _roundtrip(
            "x = {'a': [1, 2], 'b': (3, 4)}\nprint(x['a'])\nprint(x['b'])\n"
        )

    def test_chained_comparison(self) -> None:
        _roundtrip(
            "x = 5\nprint(1 < x < 10)\nprint(10 < x < 20)\n"
        )

    def test_boolean_operators(self) -> None:
        _roundtrip(
            "print(True and False)\nprint(True or False)\nprint(not True)\n"
        )

    def test_augmented_assignment(self) -> None:
        _roundtrip(
            "x = 10\nx += 5\nx -= 3\nx *= 2\nprint(x)\n"
        )

    def test_with_statement(self) -> None:
        _roundtrip(
            "import io\n"
            "with io.StringIO('hello') as f:\n"
            "    print(f.read())\n"
        )

    def test_for_else(self) -> None:
        _roundtrip(
            "for i in range(3):\n"
            "    if i == 5:\n"
            "        break\n"
            "else:\n"
            "    print('no break')\n"
        )

    def test_raise_and_catch(self) -> None:
        _roundtrip(
            "try:\n"
            "    raise ValueError('oops')\n"
            "except ValueError as e:\n"
            "    print('caught', str(e))\n"
        )

    def test_closure(self) -> None:
        _roundtrip(
            "def make_counter(start):\n"
            "    count = [start]\n"
            "    def inc():\n"
            "        count[0] += 1\n"
            "        return count[0]\n"
            "    return inc\n"
            "c = make_counter(0)\n"
            "print(c())\n"
            "print(c())\n"
        )

    def test_del_statement(self) -> None:
        _roundtrip(
            "x = [1, 2, 3]\ndel x[1]\nprint(x)\n"
        )

    def test_assert_pass(self) -> None:
        _roundtrip("assert 1 + 1 == 2\nprint('ok')\n")

    def test_two_classes(self) -> None:
        _roundtrip(
            "class Greeter:\n"
            "    def greet(self):\n"
            "        return 'hello'\n"
            "class Farewell:\n"
            "    def bye(self):\n"
            "        return 'bye'\n"
            "print(Greeter().greet())\n"
            "print(Farewell().bye())\n"
        )

    def test_multiple_return(self) -> None:
        _roundtrip(
            "def minmax(xs):\n"
            "    return min(xs), max(xs)\n"
            "a, b = minmax([3, 1, 4, 1, 5])\n"
            "print(a, b)\n"
        )

    def test_generator(self) -> None:
        _roundtrip(
            "def gen(n):\n"
            "    for i in range(n):\n"
            "        yield i * i\n"
            "print(list(gen(5)))\n"
        )

    def test_higher_order_function(self) -> None:
        _roundtrip(
            "def make_greeter(greeting):\n"
            "    def greeter(name):\n"
            "        return greeting + ' ' + name\n"
            "    return greeter\n"
            "hi = make_greeter('hi')\n"
            "print(hi('world'))\n"
        )

    def test_empty_collections(self) -> None:
        _roundtrip(
            "print([])\nprint({})\nprint(())\n"
        )

    def test_slicing(self) -> None:
        _roundtrip(
            "xs = [0, 1, 2, 3, 4]\n"
            "print(xs[1:3])\n"
            "print(xs[::2])\n"
            "print(xs[::-1])\n"
        )

    def test_multiline_string_value(self) -> None:
        _roundtrip(
            "x = 'line1\\nline2'\nprint(x)\n"
        )

    def test_none_true_false(self) -> None:
        _roundtrip("print(None)\nprint(True)\nprint(False)\n")

    def test_complex_expression(self) -> None:
        _roundtrip("print((1 + 2) * (3 - 4) / 5)\n")

    def test_nested_function(self) -> None:
        _roundtrip(
            "def outer(x):\n"
            "    def inner(y):\n"
            "        return x + y\n"
            "    return inner\n"
            "f = outer(10)\n"
            "print(f(5))\n"
        )

    def test_ellipsis(self) -> None:
        # Snail doesn't support ... literal; we use Ellipsis name
        snail_source = translate("x = ...\n")
        assert "Ellipsis" in snail_source

    def test_annotation_stripped_roundtrip(self) -> None:
        _roundtrip("x: int = 5\nprint(x)\n")

    def test_is_and_in(self) -> None:
        _roundtrip(
            "print(None is None)\n"
            "print(1 is not None)\n"
            "print(1 in [1, 2])\n"
            "print(3 not in [1, 2])\n"
        )

    def test_while_else(self) -> None:
        _roundtrip(
            "i = 0\n"
            "while i < 3:\n"
            "    i += 1\n"
            "else:\n"
            "    print('done', i)\n"
        )


# ---------------------------------------------------------------------------
# Compilation-only tests (verify translated code at least parses)
# ---------------------------------------------------------------------------


class TestCompilationOnly:
    """Verify that translated Snail code compiles without errors."""

    def _compiles(self, python_source: str) -> None:
        snail_source = translate(python_source)
        # This will raise if the Snail code doesn't parse/compile
        snail.compile_ast(snail_source)

    def test_empty_program(self) -> None:
        self._compiles("")

    def test_comment_only(self) -> None:
        # Comments are stripped by ast.parse, result is empty
        self._compiles("# just a comment\n")

    def test_nested_class(self) -> None:
        self._compiles(
            "class Outer:\n"
            "    class Inner:\n"
            "        def method(self):\n"
            "            return 1\n"
        )

    def test_complex_defaults(self) -> None:
        self._compiles(
            "def f(x=[1,2], y={'a': 1}, z=(1,)):\n    pass\n"
        )

    def test_multiline_dict(self) -> None:
        self._compiles(
            "x = {\n    'a': 1,\n    'b': 2,\n    'c': 3,\n}\n"
        )

    def test_chained_calls(self) -> None:
        self._compiles(
            '"hello world".split().pop()\n'
        )

    def test_starred_assignment(self) -> None:
        self._compiles("first, *rest = [1, 2, 3, 4]\n")

    def test_conditional_import(self) -> None:
        self._compiles(
            "try:\n    import ujson as json\nexcept ImportError:\n    import json\n"
        )


# ---------------------------------------------------------------------------
# Keyword mangling tests
# ---------------------------------------------------------------------------


class TestKeywordMangling:
    """Test that Snail-only keywords are mangled in translated output."""

    def test_variable_named_awk(self) -> None:
        result = translate("awk = 1\nprint(awk)\n")
        assert "awk_" in result
        assert "awk_ = 1" in result
        assert "print(awk_)" in result

    def test_attribute_named_awk(self) -> None:
        result = translate("self.awk = 1\n")
        assert "self.awk_" in result

    def test_function_named_let(self) -> None:
        result = translate("def let():\n    pass\n")
        assert "def let_(" in result

    def test_parameter_named_xargs(self) -> None:
        result = translate("def f(xargs):\n    pass\n")
        assert "def f(xargs_)" in result

    def test_keyword_argument_awk(self) -> None:
        result = translate("f(awk=1)\n")
        assert "awk_=1" in result

    def test_class_named_let(self) -> None:
        result = translate("class let:\n    pass\n")
        assert "class let_" in result

    def test_bare_underscore_renamed(self) -> None:
        result = translate("_ = 1\nprint(_)\n")
        assert "__ = 1" in result
        assert "print(__)" in result

    def test_except_handler_named_awk(self) -> None:
        result = translate(
            "try:\n    pass\nexcept Exception as awk:\n    print(awk)\n"
        )
        assert "as awk_" in result
        assert "print(awk_)" in result

    def test_vararg_named_awk(self) -> None:
        result = translate("def f(*awk):\n    pass\n")
        assert "*awk_" in result

    def test_kwarg_named_let(self) -> None:
        result = translate("def f(**let):\n    pass\n")
        assert "**let_" in result

    def test_kwonly_named_xargs(self) -> None:
        result = translate("def f(*, xargs):\n    pass\n")
        assert "xargs_" in result

    # -- import rewrites -------------------------------------------------------

    def test_import_keyword_module(self) -> None:
        result = translate("import awk\n")
        assert '__import__("awk")' in result
        assert "awk_ =" in result

    def test_import_keyword_module_as(self) -> None:
        result = translate("import awk as x\n")
        assert '__import__("awk")' in result
        assert "x =" in result

    def test_from_keyword_module_import(self) -> None:
        result = translate("from awk import bar\n")
        assert '__import__("awk")' in result
        assert 'getattr(' in result
        assert '"bar"' in result

    def test_from_normal_module_import_keyword(self) -> None:
        result = translate("from foo import awk\n")
        assert 'getattr(__import__("foo"), "awk")' in result
        assert "awk_ =" in result

    def test_from_normal_module_import_keyword_as(self) -> None:
        result = translate("from foo import awk as x\n")
        assert 'getattr(__import__("foo"), "awk")' in result
        assert "x =" in result

    def test_from_keyword_module_star_raises(self) -> None:
        with pytest.raises(Py2SnailError, match="star import"):
            translate("from awk import *\n")

    def test_import_asname_mangled(self) -> None:
        result = translate("import foo as awk\n")
        assert "import foo as awk_" in result

    def test_from_import_asname_mangled(self) -> None:
        result = translate("from foo import bar as awk\n")
        assert "from foo import bar as awk_" in result

    def test_normal_import_unchanged(self) -> None:
        result = translate("import os\n")
        assert "import os" in result

    # -- compilation tests (verify translated code parses as Snail) ------------

    def test_keyword_variable_compiles(self) -> None:
        snail_source = translate("awk = 1\nprint(awk)\n")
        snail.compile_ast(snail_source)

    def test_keyword_attribute_compiles(self) -> None:
        snail_source = translate(
            "class Foo:\n"
            "    def __init__(self):\n"
            "        self.awk = 1\n"
        )
        snail.compile_ast(snail_source)

    def test_keyword_function_compiles(self) -> None:
        snail_source = translate("def let(x):\n    return x\n")
        snail.compile_ast(snail_source)

    # -- roundtrip tests -------------------------------------------------------

    def test_roundtrip_keyword_variable(self) -> None:
        _roundtrip("awk = 42\nprint(awk)\n")

    def test_roundtrip_keyword_attribute(self) -> None:
        _roundtrip(
            "class Foo:\n"
            "    def __init__(self):\n"
            "        self.awk = 1\n"
            "    def get(self):\n"
            "        return self.awk\n"
            "print(Foo().get())\n"
        )

    def test_roundtrip_bare_underscore(self) -> None:
        _roundtrip("_ = 99\nprint(_)\n")

    def test_roundtrip_starred_in_list(self) -> None:
        _roundtrip("print([*[1, 2], 3])\n")

    def test_roundtrip_dict_unpacking(self) -> None:
        _roundtrip("a = {'x': 1}\nprint({**a, 'y': 2})\n")
