from __future__ import annotations

import operator as _op

_OPS = {
    "+": _op.add,
    "-": _op.sub,
    "*": _op.mul,
    "/": _op.truediv,
    "//": _op.floordiv,
    "%": _op.mod,
    "**": _op.pow,
}


def _apply_op(left, right, op: str):
    try:
        func = _OPS[op]
    except KeyError as exc:
        raise ValueError(f"unknown augmented op: {op}") from exc
    return func(left, right)


def __snail_incr_attr(obj, attr: str, delta: int, pre: bool):
    old = getattr(obj, attr)
    new = old + delta
    setattr(obj, attr, new)
    return new if pre else old


def __snail_incr_index(obj, index, delta: int, pre: bool):
    old = obj[index]
    new = old + delta
    obj[index] = new
    return new if pre else old


def __snail_aug_attr(obj, attr: str, value, op: str):
    old = getattr(obj, attr)
    new = _apply_op(old, value, op)
    setattr(obj, attr, new)
    return new


def __snail_aug_index(obj, index, value, op: str):
    old = obj[index]
    new = _apply_op(old, value, op)
    obj[index] = new
    return new
