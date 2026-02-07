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


def _incr(get_value, set_value, delta: int, pre: bool):
    old = get_value()
    new = old + delta
    set_value(new)
    return new if pre else old


def _aug(get_value, set_value, value, op: str):
    old = get_value()
    new = _apply_op(old, value, op)
    set_value(new)
    return new


def __snail_incr_attr(obj, attr: str, delta: int, pre: bool):
    def get_value():
        return getattr(obj, attr)

    def set_value(new):
        setattr(obj, attr, new)

    return _incr(get_value, set_value, delta, pre)


def __snail_incr_index(obj, index, delta: int, pre: bool):
    def get_value():
        return obj[index]

    def set_value(new):
        obj[index] = new

    return _incr(get_value, set_value, delta, pre)


def __snail_aug_attr(obj, attr: str, value, op: str):
    def get_value():
        return getattr(obj, attr)

    def set_value(new):
        setattr(obj, attr, new)

    return _aug(get_value, set_value, value, op)


def __snail_aug_index(obj, index, value, op: str):
    def get_value():
        return obj[index]

    def set_value(new):
        obj[index] = new

    return _aug(get_value, set_value, value, op)
