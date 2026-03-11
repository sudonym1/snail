"""Translate Python source code to equivalent Snail source code.

Uses Python's ``ast`` module to parse the input and an ``ast.NodeVisitor``
subclass to emit Snail syntax.  By default, idiomatic Snail transforms are
applied (``++``/``--``, compact try ``?``, auto-import elision).  Pass
``idiomatic=False`` (or use the ``--mechanical`` CLI flag) for a purely
mechanical translation that preserves round-trip semantics without shortcuts.

Unsupported Python features (raise Py2SnailError)
--------------------------------------------------
- async/await: async def, async for, async with, await, async comprehensions
- match/case statement (Python 3.10+)
- except* / exception groups (Python 3.11+)
- type aliases: type X = ... (Python 3.12+)
- global and nonlocal statements
- Class metaclasses: class Foo(metaclass=Bar)
- Positional-only parameter separator: def f(a, /, b)
- Matrix multiply operator: a @ b, a @= b
- Dict unpacking in dict literals: {**a, **b}

Lossy but semantics-preserving transformations
----------------------------------------------
- Comments are dropped (ast.parse discards them)
- String delimiter choice is lost (always emits double quotes)
- Raw string prefix is lost (ast normalizes the content)
- Type annotations are stripped (def f(x: int) -> def f(x))
- Annotation-only statements are skipped (x: int -> nothing)
- Multi-target assignment is desugared (a = b = x -> b = x; a = b)
- Walrus operator uses block expression (x := expr -> { x = expr; x }).
  Works in if conditions and general expressions; does NOT work in while
  conditions (block re-evaluates with stale bindings each iteration)
- Ellipsis literal ... is emitted as the name Ellipsis
- Step slices use slice() builtin (x[::2] -> x[slice(None, None, 2)])
- Identifiers that collide with Snail-only keywords (let, awk, xargs) get a
  trailing underscore (PEP 8 convention): awk -> awk_, let -> let_, etc.
- Bare ``_`` is renamed to ``__`` (``_`` is Snail's pipeline placeholder)
- Imports where a keyword appears in a grammar-parsed position are rewritten
  using ``__import__`` / ``getattr`` to avoid parse failures

Known Snail runtime issues
--------------------------
- Augmented assignment on attributes inside classes (self.x += 1)
  fails due to Snail runtime name-mangling bug with __snail_aug_attr
"""

from __future__ import annotations

import ast
import sys


class Py2SnailError(Exception):
    """Raised when an unsupported Python construct is encountered."""


# Keyword mangling ------------------------------------------------------------

# Identifiers that are keywords in Snail but not in Python.
# Bare "_" is also reserved (Snail pipeline placeholder).
_SNAIL_ONLY_KEYWORDS = frozenset({"let", "awk", "xargs"})


def _mangle(name: str) -> str:
    """Append '_' to identifiers that collide with Snail keywords."""
    if name in _SNAIL_ONLY_KEYWORDS:
        return name + "_"
    if name == "_":
        return "__"
    return name


def _needs_import_rewrite(name: str) -> bool:
    """Return True if *name* (or any dotted component) is a Snail keyword."""
    return any(part in _SNAIL_ONLY_KEYWORDS or part == "_" for part in name.split("."))


# Auto-import tables (must match snail.runtime.AUTO_IMPORT_NAMES) -------------
# Whole-module imports that Snail auto-imports: import X
_AUTO_IMPORT_MODULES = frozenset({"sys", "os"})
# Attribute imports that Snail auto-imports: from M import X  →  name → module
_AUTO_IMPORT_ATTRS: dict[str, str] = {
    "Path": "pathlib",
    "sleep": "time",
    "pprint": "pprint",
    "defaultdict": "collections",
}

# Operator maps ---------------------------------------------------------------

_BINOP = {
    ast.Add: "+",
    ast.Sub: "-",
    ast.Mult: "*",
    ast.Div: "/",
    ast.FloorDiv: "//",
    ast.Mod: "%",
    ast.Pow: "**",
    ast.LShift: "<<",
    ast.RShift: ">>",
    ast.BitOr: "|",
    ast.BitXor: "^",
    ast.BitAnd: "&",
}

_UNARYOP = {
    ast.UAdd: "+",
    ast.USub: "-",
    ast.Not: "not ",
    ast.Invert: "~",
}

_CMPOP = {
    ast.Eq: "==",
    ast.NotEq: "!=",
    ast.Lt: "<",
    ast.LtE: "<=",
    ast.Gt: ">",
    ast.GtE: ">=",
    ast.Is: "is",
    ast.IsNot: "is not",
    ast.In: "in",
    ast.NotIn: "not in",
}

_BOOLOP = {
    ast.And: "and",
    ast.Or: "or",
}

_AUGOP = {
    ast.Add: "+=",
    ast.Sub: "-=",
    ast.Mult: "*=",
    ast.Div: "/=",
    ast.FloorDiv: "//=",
    ast.Mod: "%=",
    ast.Pow: "**=",
    ast.LShift: "<<=",
    ast.RShift: ">>=",
    ast.BitOr: "|=",
    ast.BitXor: "^=",
    ast.BitAnd: "&=",
}

# Precedence table for parenthesization
_PRECEDENCE: dict[type, int] = {
    ast.Lambda: 1,
    ast.IfExp: 2,
    ast.Or: 3,  # BoolOp Or
    ast.And: 4,  # BoolOp And
    ast.Not: 5,  # UnaryOp Not
    ast.Compare: 6,
    ast.BitOr: 7,
    ast.BitXor: 8,
    ast.BitAnd: 9,
    ast.LShift: 10,
    ast.RShift: 10,
    ast.Add: 11,
    ast.Sub: 11,
    ast.Mult: 12,
    ast.Div: 12,
    ast.FloorDiv: 12,
    ast.Mod: 12,
    ast.UAdd: 13,
    ast.USub: 13,
    ast.Invert: 13,
    ast.Pow: 14,
    ast.Await: 15,
    ast.Subscript: 16,
    ast.Attribute: 16,
    ast.Call: 16,
}


def _prec(node: ast.expr) -> int:
    """Return a precedence value for *node* (higher binds tighter)."""
    if isinstance(node, ast.BoolOp):
        return _PRECEDENCE.get(type(node.op), 0)
    if isinstance(node, ast.BinOp):
        return _PRECEDENCE.get(type(node.op), 0)
    if isinstance(node, ast.UnaryOp):
        return _PRECEDENCE.get(type(node.op), 0)
    return _PRECEDENCE.get(type(node), 20)


class SnailUnparser(ast.NodeVisitor):
    """AST visitor that emits Snail source text."""

    def __init__(self, *, idiomatic: bool = True) -> None:
        self._indent = 0
        self._result: list[str] = []
        self._idiomatic = idiomatic

    # -- helpers --------------------------------------------------------------

    def _i(self) -> str:
        """Current indentation string."""
        return "    " * self._indent

    def _write(self, text: str) -> None:
        self._result.append(text)

    def _line(self, text: str) -> None:
        self._write(f"{self._i()}{text}\n")

    def _block(self, body: list[ast.stmt]) -> None:
        self._write(" {\n")
        self._indent += 1
        for stmt in body:
            self.visit(stmt)
        self._indent -= 1
        self._write(f"{self._i()}}}\n")

    def _expr(self, node: ast.expr) -> str:
        """Return the Snail source for an expression node."""
        return self._visit_expr(node)

    def _visit_expr(self, node: ast.expr) -> str:
        method = "visit_" + node.__class__.__name__
        visitor = getattr(self, method, None)
        if visitor is None:
            raise Py2SnailError(f"Unsupported expression: {node.__class__.__name__}")
        return visitor(node)

    def _paren(self, node: ast.expr, parent_prec: int) -> str:
        """Parenthesise *node* if its precedence is lower than *parent_prec*."""
        s = self._expr(node)
        if _prec(node) < parent_prec:
            return f"({s})"
        return s

    # -- statements -----------------------------------------------------------

    def visit_Module(self, node: ast.Module) -> None:
        for stmt in node.body:
            self.visit(stmt)

    def _emit_decorators(self, decorator_list: list[ast.expr]) -> None:
        for dec in decorator_list:
            self._line(f"@{self._expr(dec)}")

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._emit_decorators(node.decorator_list)
        args = self._format_args(node.args)
        self._write(f"{self._i()}def {_mangle(node.name)}({args})")
        self._block(node.body)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        raise Py2SnailError("async functions are not supported by Snail")

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        if node.keywords:
            raise Py2SnailError("metaclasses are not supported by Snail")
        self._emit_decorators(node.decorator_list)
        bases = ""
        if node.bases:
            bases = "(" + ", ".join(self._expr(b) for b in node.bases) + ")"
        self._write(f"{self._i()}class {_mangle(node.name)}{bases}")
        self._block(node.body)

    def visit_Return(self, node: ast.Return) -> None:
        if node.value is None:
            self._line("return")
        else:
            self._line(f"return {self._expr(node.value)}")

    def visit_Delete(self, node: ast.Delete) -> None:
        targets = ", ".join(self._expr(t) for t in node.targets)
        self._line(f"del {targets}")

    def visit_Assign(self, node: ast.Assign) -> None:
        value = self._expr(node.value)
        if len(node.targets) == 1:
            self._line(f"{self._expr(node.targets[0])} = {value}")
        else:
            # Multi-target: a = b = x  →  b = x; a = b
            # Desugar right to left
            prev = value
            lines: list[str] = []
            for target in reversed(node.targets):
                t = self._expr(target)
                lines.append(f"{t} = {prev}")
                prev = t
            for ln in reversed(lines):
                self._line(ln)

    def visit_AugAssign(self, node: ast.AugAssign) -> None:
        # Idiomatic: x += 1 → x++, x -= 1 → x--
        if self._idiomatic and isinstance(node.target, ast.Name):
            if (
                isinstance(node.value, ast.Constant)
                and type(node.value.value) is int
                and node.value.value == 1
            ):
                if isinstance(node.op, ast.Add):
                    self._line(f"{_mangle(node.target.id)}++")
                    return
                if isinstance(node.op, ast.Sub):
                    self._line(f"{_mangle(node.target.id)}--")
                    return
        op = _AUGOP.get(type(node.op))
        if op is None:
            raise Py2SnailError(
                f"Unsupported augmented assignment operator: {node.op.__class__.__name__}"
            )
        self._line(f"{self._expr(node.target)} {op} {self._expr(node.value)}")

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        # Strip type annotations
        if node.value is not None:
            self._line(f"{self._expr(node.target)} = {self._expr(node.value)}")
        # else: annotation-only statement, skip entirely

    def visit_For(self, node: ast.For) -> None:
        self._write(
            f"{self._i()}for {self._expr(node.target)} in {self._expr(node.iter)}"
        )
        self._block(node.body)
        if node.orelse:
            self._write(f"{self._i()}else")
            self._block(node.orelse)

    def visit_AsyncFor(self, node: ast.AsyncFor) -> None:
        raise Py2SnailError("async for is not supported by Snail")

    def visit_While(self, node: ast.While) -> None:
        self._write(f"{self._i()}while {self._expr(node.test)}")
        self._block(node.body)
        if node.orelse:
            self._write(f"{self._i()}else")
            self._block(node.orelse)

    def visit_If(self, node: ast.If) -> None:
        self._write(f"{self._i()}if {self._expr(node.test)}")
        self._block(node.body)
        orelse = node.orelse
        while orelse:
            if len(orelse) == 1 and isinstance(orelse[0], ast.If):
                elif_node = orelse[0]
                self._write(f"{self._i()}elif {self._expr(elif_node.test)}")
                self._block(elif_node.body)
                orelse = elif_node.orelse
            else:
                self._write(f"{self._i()}else")
                self._block(orelse)
                break

    def visit_With(self, node: ast.With) -> None:
        items = []
        for item in node.items:
            s = self._expr(item.context_expr)
            if item.optional_vars is not None:
                s += f" as {self._expr(item.optional_vars)}"
            items.append(s)
        self._write(f"{self._i()}with {', '.join(items)}")
        self._block(node.body)

    def visit_AsyncWith(self, node: ast.AsyncWith) -> None:
        raise Py2SnailError("async with is not supported by Snail")

    def visit_Raise(self, node: ast.Raise) -> None:
        if node.exc is None:
            self._line("raise")
        elif node.cause is not None:
            self._line(
                f"raise {self._expr(node.exc)} from {self._expr(node.cause)}"
            )
        else:
            self._line(f"raise {self._expr(node.exc)}")

    def _try_compact_try(self, node: ast.Try) -> bool:
        """Attempt to emit a compact try expression. Returns True if emitted."""
        # Must have exactly 1 handler, no else/finally
        if len(node.handlers) != 1 or node.orelse or node.finalbody:
            return False
        handler = node.handlers[0]
        # Handler must be bare except or except Exception (not specific types)
        if handler.type is not None:
            if not (isinstance(handler.type, ast.Name) and handler.type.id == "Exception"):
                return False
        # Try and handler bodies must each have exactly 1 statement
        if len(node.body) != 1 or len(handler.body) != 1:
            return False
        try_stmt = node.body[0]
        except_stmt = handler.body[0]
        # Pattern C: try { expr } except { pass }
        if isinstance(try_stmt, ast.Expr) and isinstance(except_stmt, ast.Pass):
            self._line(f"{self._expr(try_stmt.value)}?")
            return True
        # Pattern A/B: try { x = expr } except { x = fallback }
        if (
            isinstance(try_stmt, ast.Assign)
            and len(try_stmt.targets) == 1
            and isinstance(except_stmt, ast.Assign)
            and len(except_stmt.targets) == 1
        ):
            try_target = self._expr(try_stmt.targets[0])
            except_target = self._expr(except_stmt.targets[0])
            if try_target != except_target:
                return False
            expr = self._expr(try_stmt.value)
            fallback_value = except_stmt.value
            # Pattern A: fallback is None
            if isinstance(fallback_value, ast.Constant) and fallback_value.value is None:
                self._line(f"{try_target} = {expr}?")
                return True
            # Pattern B: fallback is any other expression
            fallback = self._expr(fallback_value)
            self._line(f"{try_target} = {expr}:{fallback}?")
            return True
        return False

    def visit_Try(self, node: ast.Try) -> None:
        if self._idiomatic and self._try_compact_try(node):
            return
        self._write(f"{self._i()}try")
        self._block(node.body)
        for handler in node.handlers:
            if handler.type is None:
                self._write(f"{self._i()}except")
            elif handler.name:
                self._write(
                    f"{self._i()}except {self._expr(handler.type)} as {_mangle(handler.name)}"
                )
            else:
                self._write(f"{self._i()}except {self._expr(handler.type)}")
            self._block(handler.body)
        if node.orelse:
            self._write(f"{self._i()}else")
            self._block(node.orelse)
        if node.finalbody:
            self._write(f"{self._i()}finally")
            self._block(node.finalbody)

    def visit_Assert(self, node: ast.Assert) -> None:
        if node.msg is not None:
            self._line(
                f"assert {self._expr(node.test)}, {self._expr(node.msg)}"
            )
        else:
            self._line(f"assert {self._expr(node.test)}")

    def visit_Import(self, node: ast.Import) -> None:
        aliases = list(node.names)
        # Idiomatic: elide auto-imported modules (only bare imports, no alias)
        if self._idiomatic:
            aliases = [
                a for a in aliases
                if not (a.asname is None and a.name in _AUTO_IMPORT_MODULES)
            ]
            if not aliases:
                return
        # If any name needs rewriting, emit each alias on its own line
        any_keyword = any(
            _needs_import_rewrite(alias.name)
            or (alias.asname and _needs_import_rewrite(alias.asname))
            for alias in aliases
        )
        if any_keyword:
            for alias in aliases:
                if _needs_import_rewrite(alias.name):
                    bound = _mangle(alias.asname or alias.name.split(".")[0])
                    self._line(f'{bound} = __import__("{alias.name}")')
                elif alias.asname:
                    self._line(f"import {alias.name} as {_mangle(alias.asname)}")
                else:
                    self._line(f"import {alias.name}")
        else:
            parts = []
            for alias in aliases:
                if alias.asname:
                    parts.append(f"{alias.name} as {_mangle(alias.asname)}")
                else:
                    parts.append(alias.name)
            self._line(f"import {', '.join(parts)}")

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        module = node.module or ""
        prefix = "." * (node.level or 0)
        full_module = f"{prefix}{module}"

        # Idiomatic: elide if ALL names are auto-imports from this module
        if self._idiomatic and not node.level and node.names:
            all_auto = all(
                alias.asname is None
                and alias.name in _AUTO_IMPORT_ATTRS
                and _AUTO_IMPORT_ATTRS[alias.name] == module
                for alias in node.names
            )
            if all_auto:
                return

        # Check if the module path contains a Snail keyword
        module_has_keyword = _needs_import_rewrite(module) if module else False

        if module_has_keyword:
            if node.level:
                raise Py2SnailError(
                    f"relative import from keyword module '{full_module}' "
                    "cannot be mechanically rewritten"
                )
            for alias in node.names:
                if alias.name == "*":
                    raise Py2SnailError(
                        f"star import from keyword module '{module}' "
                        "cannot be mechanically rewritten"
                    )
                bound = _mangle(alias.asname or alias.name)
                self._line(
                    f'{bound} = getattr(__import__("{module}"), "{alias.name}")'
                )
            return

        # Check if any imported names are Snail keywords
        any_name_keyword = any(
            _needs_import_rewrite(alias.name) for alias in node.names
        )

        if any_name_keyword:
            # Must rewrite each name that conflicts
            for alias in node.names:
                if _needs_import_rewrite(alias.name):
                    bound = _mangle(alias.asname or alias.name)
                    self._line(
                        f'{bound} = getattr(__import__("{module}"), "{alias.name}")'
                    )
                elif alias.asname:
                    self._line(
                        f"from {full_module} import {alias.name} as {_mangle(alias.asname)}"
                    )
                else:
                    self._line(f"from {full_module} import {alias.name}")
            return

        # No keyword conflicts — normal import
        names = []
        for alias in node.names:
            if alias.asname:
                names.append(f"{alias.name} as {_mangle(alias.asname)}")
            else:
                names.append(alias.name)
        if len(names) == 1:
            self._line(f"from {full_module} import {names[0]}")
        else:
            self._line(f"from {full_module} import ({', '.join(names)})")

    def visit_Global(self, node: ast.Global) -> None:
        raise Py2SnailError("global statement is not supported by Snail")

    def visit_Nonlocal(self, node: ast.Nonlocal) -> None:
        raise Py2SnailError("nonlocal statement is not supported by Snail")

    def visit_Expr(self, node: ast.Expr) -> None:
        self._line(self._expr(node.value))

    def visit_Pass(self, node: ast.Pass) -> None:
        self._line("pass")

    def visit_Break(self, node: ast.Break) -> None:
        self._line("break")

    def visit_Continue(self, node: ast.Continue) -> None:
        self._line("continue")

    # Python 3.10+ TryStar
    def visit_TryStar(self, node: object) -> None:
        raise Py2SnailError("except* (exception groups) not supported by Snail")

    # Python 3.10+ match
    def visit_Match(self, node: object) -> None:
        raise Py2SnailError("match statement not supported by Snail")

    # Python 3.12+ type alias
    def visit_TypeAlias(self, node: object) -> None:
        raise Py2SnailError("type alias statement not supported by Snail")

    # -- expressions ----------------------------------------------------------

    def visit_BoolOp(self, node: ast.BoolOp) -> str:
        op = _BOOLOP[type(node.op)]
        prec = _prec(node)
        parts = [self._paren(v, prec) for v in node.values]
        return f" {op} ".join(parts)

    def visit_NamedExpr(self, node: ast.NamedExpr) -> str:
        # x := expr  →  { x = expr; x }
        target = self._expr(node.target)
        value = self._expr(node.value)
        return "{ " + target + " = " + value + "; " + target + " }"

    def visit_BinOp(self, node: ast.BinOp) -> str:
        op = _BINOP.get(type(node.op))
        if op is None:
            raise Py2SnailError(
                f"Unsupported binary operator: {node.op.__class__.__name__}"
            )
        prec = _prec(node)
        # For right-associative ** we need prec+1 on left, prec on right
        if isinstance(node.op, ast.Pow):
            left = self._paren(node.left, prec + 1)
            right = self._paren(node.right, prec)
        else:
            left = self._paren(node.left, prec)
            right = self._paren(node.right, prec + 1)
        return f"{left} {op} {right}"

    def visit_UnaryOp(self, node: ast.UnaryOp) -> str:
        op = _UNARYOP.get(type(node.op))
        if op is None:
            raise Py2SnailError(
                f"Unsupported unary operator: {node.op.__class__.__name__}"
            )
        prec = _prec(node)
        operand = self._paren(node.operand, prec)
        return f"{op}{operand}"

    def visit_Lambda(self, node: ast.Lambda) -> str:
        args = self._format_args(node.args)
        body = self._expr(node.body)
        return f"def({args}) {{ {body} }}"

    def visit_IfExp(self, node: ast.IfExp) -> str:
        test = self._expr(node.test)
        body = self._expr(node.body)
        orelse = self._expr(node.orelse)
        return f"if {test} {{ {body} }} else {{ {orelse} }}"

    def visit_Dict(self, node: ast.Dict) -> str:
        if not node.keys and not node.values:
            return "%{}"
        pairs = []
        for k, v in zip(node.keys, node.values):
            if k is None:
                pairs.append(f"**{self._expr(v)}")
            else:
                pairs.append(f"{self._expr(k)}: {self._expr(v)}")
        return "%{" + ", ".join(pairs) + "}"

    def visit_Set(self, node: ast.Set) -> str:
        elts = ", ".join(self._expr(e) for e in node.elts)
        return "#{" + elts + "}"

    def visit_ListComp(self, node: ast.ListComp) -> str:
        elt = self._expr(node.elt)
        comps = self._format_comprehensions(node.generators)
        return f"[{elt} {comps}]"

    def visit_SetComp(self, node: ast.SetComp) -> str:
        elt = self._expr(node.elt)
        comps = self._format_comprehensions(node.generators)
        return "#{{" + f"{elt} {comps}" + "}}"

    def visit_DictComp(self, node: ast.DictComp) -> str:
        key = self._expr(node.key)
        value = self._expr(node.value)
        comps = self._format_comprehensions(node.generators)
        return "%{" + f"{key}: {value} {comps}" + "}"

    def visit_GeneratorExp(self, node: ast.GeneratorExp) -> str:
        elt = self._expr(node.elt)
        comps = self._format_comprehensions(node.generators)
        return f"({elt} {comps})"

    def _format_comprehensions(self, generators: list[ast.comprehension]) -> str:
        parts = []
        for gen in generators:
            target = self._expr(gen.target)
            iter_ = self._expr(gen.iter)
            s = f"for {target} in {iter_}"
            for if_ in gen.ifs:
                s += f" if {self._expr(if_)}"
            if gen.is_async:
                raise Py2SnailError("async comprehensions not supported by Snail")
            parts.append(s)
        return " ".join(parts)

    def visit_Await(self, node: ast.Await) -> str:
        raise Py2SnailError("await is not supported by Snail")

    def visit_Yield(self, node: ast.Yield) -> str:
        if node.value is None:
            return "yield"
        return f"yield {self._expr(node.value)}"

    def visit_YieldFrom(self, node: ast.YieldFrom) -> str:
        return f"yield from {self._expr(node.value)}"

    def visit_Compare(self, node: ast.Compare) -> str:
        parts = [self._paren(node.left, _prec(node))]
        for op, comparator in zip(node.ops, node.comparators):
            op_str = _CMPOP.get(type(op))
            if op_str is None:
                raise Py2SnailError(
                    f"Unsupported comparison: {op.__class__.__name__}"
                )
            parts.append(op_str)
            parts.append(self._paren(comparator, _prec(node) + 1))
        return " ".join(parts)

    def visit_Call(self, node: ast.Call) -> str:
        func = self._paren(node.func, _prec(node))
        args = []
        for a in node.args:
            args.append(self._expr(a))
        for kw in node.keywords:
            if kw.arg is None:
                args.append(f"**{self._expr(kw.value)}")
            else:
                args.append(f"{_mangle(kw.arg)}={self._expr(kw.value)}")
        return f"{func}({', '.join(args)})"

    def visit_FormattedValue(self, node: ast.FormattedValue) -> str:
        # Used inside JoinedStr
        inner = self._expr(node.value)
        conversion = ""
        if node.conversion and node.conversion != -1:
            conversion = f"!{chr(node.conversion)}"
        fmt = ""
        if node.format_spec:
            fmt = f":{self._format_spec_str(node.format_spec)}"
        return "{" + inner + conversion + fmt + "}"

    def _format_spec_str(self, spec: ast.expr) -> str:
        """Convert a format_spec (a JoinedStr) to its string content."""
        if isinstance(spec, ast.JoinedStr):
            parts = []
            for v in spec.values:
                if isinstance(v, ast.Constant) and isinstance(v.value, str):
                    parts.append(v.value)
                else:
                    parts.append(self._visit_expr(v))
            return "".join(parts)
        return self._expr(spec)

    def visit_JoinedStr(self, node: ast.JoinedStr) -> str:
        # f-string → Snail interpolated string (just drop the f prefix)
        parts = []
        for v in node.values:
            if isinstance(v, ast.Constant) and isinstance(v.value, str):
                # Literal text — escape backslashes, quotes, and braces
                # (braces must be doubled so Snail doesn't interpolate them)
                escaped = v.value.replace("\\", "\\\\").replace('"', '\\"')
                # Newlines etc.
                escaped = (
                    escaped.replace("\n", "\\n")
                    .replace("\r", "\\r")
                    .replace("\t", "\\t")
                )
                escaped = escaped.replace("{", "{{").replace("}", "}}")
                parts.append(escaped)
            elif isinstance(v, ast.FormattedValue):
                parts.append(self.visit_FormattedValue(v))
            else:
                parts.append(self._expr(v))
        return '"' + "".join(parts) + '"'

    def visit_Constant(self, node: ast.Constant) -> str:
        if isinstance(node.value, str):
            # Escape literal braces for Snail (non-f-string)
            escaped = node.value.replace("\\", "\\\\").replace('"', '\\"')
            escaped = (
                escaped.replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t")
            )
            # Escape braces so Snail doesn't treat them as interpolation
            escaped = escaped.replace("{", "{{").replace("}", "}}")
            return f'"{escaped}"'
        if isinstance(node.value, bytes):
            return repr(node.value)
        if isinstance(node.value, bool):
            return "True" if node.value else "False"
        if node.value is None:
            return "None"
        if isinstance(node.value, (int, float, complex)):
            return repr(node.value)
        if isinstance(node.value, type(...)):
            return "Ellipsis"
        return repr(node.value)

    def visit_Attribute(self, node: ast.Attribute) -> str:
        value = self._paren(node.value, _prec(node))
        return f"{value}.{_mangle(node.attr)}"

    def visit_Subscript(self, node: ast.Subscript) -> str:
        value = self._paren(node.value, _prec(node))
        slice_node = node.slice
        # Python 3.8 wraps subscript indices in ast.Index
        if hasattr(ast, "Index") and isinstance(slice_node, ast.Index):
            slice_node = slice_node.value  # type: ignore[attr-defined]
        sl = self._expr(slice_node)
        return f"{value}[{sl}]"

    def visit_Starred(self, node: ast.Starred) -> str:
        return f"*{self._expr(node.value)}"

    def visit_Name(self, node: ast.Name) -> str:
        return _mangle(node.id)

    def visit_List(self, node: ast.List) -> str:
        elts = ", ".join(self._expr(e) for e in node.elts)
        return f"[{elts}]"

    def visit_Tuple(self, node: ast.Tuple) -> str:
        if not node.elts:
            return "()"
        elts = ", ".join(self._expr(e) for e in node.elts)
        if len(node.elts) == 1:
            return f"({elts},)"
        return f"({elts})"

    def visit_Slice(self, node: ast.Slice) -> str:
        lower = self._expr(node.lower) if node.lower else ""
        upper = self._expr(node.upper) if node.upper else ""
        if node.step:
            # Snail doesn't support step slicing (a:b:c) syntax,
            # so use slice() builtin instead
            lower_arg = self._expr(node.lower) if node.lower else "None"
            upper_arg = self._expr(node.upper) if node.upper else "None"
            step_arg = self._expr(node.step)
            return f"slice({lower_arg}, {upper_arg}, {step_arg})"
        return f"{lower}:{upper}"

    # -- argument formatting --------------------------------------------------

    def _format_args(self, args: ast.arguments) -> str:
        """Format function arguments, stripping type annotations."""
        parts: list[str] = []
        # positional-only args (before /) and regular args
        posonlyargs = getattr(args, "posonlyargs", [])
        all_positional = list(posonlyargs) + list(args.args)
        num_defaults = len(args.defaults)
        default_offset = len(all_positional) - num_defaults

        for i, arg in enumerate(all_positional):
            s = _mangle(arg.arg)
            di = i - default_offset
            if di >= 0 and di < len(args.defaults):
                s += f"={self._expr(args.defaults[di])}"
            parts.append(s)
            # Insert / separator after the last positional-only arg
            if posonlyargs and i == len(posonlyargs) - 1:
                parts.append("/")

        if args.vararg:
            parts.append(f"*{_mangle(args.vararg.arg)}")
        elif args.kwonlyargs:
            parts.append("*")

        for i, arg in enumerate(args.kwonlyargs):
            s = _mangle(arg.arg)
            default = args.kw_defaults[i] if i < len(args.kw_defaults) else None
            if default is not None:
                s += f"={self._expr(default)}"
            parts.append(s)

        if args.kwarg:
            parts.append(f"**{_mangle(args.kwarg.arg)}")

        return ", ".join(parts)

    # -- top-level entry ------------------------------------------------------

    def unparse(self, source: str) -> str:
        """Parse *source* as Python and return equivalent Snail source."""
        tree = ast.parse(source)
        self.visit(tree)
        return "".join(self._result)


def translate(python_source: str, *, idiomatic: bool = True) -> str:
    """Translate Python source code to Snail source code."""
    return SnailUnparser(idiomatic=idiomatic).unparse(python_source)


def main() -> None:
    """CLI entry point: reads Python from stdin or file, writes Snail to stdout."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Translate Python source to Snail"
    )
    parser.add_argument(
        "file",
        nargs="?",
        help="Python source file (reads stdin if omitted)",
    )
    parser.add_argument(
        "-m",
        "--mechanical",
        action="store_true",
        help="Disable idiomatic Snail transforms (no ++, ?, auto-import elision)",
    )
    args = parser.parse_args()

    if args.file:
        with open(args.file) as f:
            source = f.read()
    else:
        source = sys.stdin.read()

    try:
        result = translate(source, idiomatic=not args.mechanical)
    except Py2SnailError as e:
        print(f"py2snail error: {e}", file=sys.stderr)
        sys.exit(1)
    except SyntaxError as e:
        print(f"Python syntax error: {e}", file=sys.stderr)
        sys.exit(1)

    sys.stdout.write(result)


if __name__ == "__main__":
    main()
