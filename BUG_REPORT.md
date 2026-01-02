# Bug Hunting Report

**Date**: 2026-01-02
**Total Tests Created**: 50
**Real Bugs Found**: 5 (10 if counting duplicate instances)

## Critical Bugs Found

### 1. Power Operator Wrong Associativity ⚠️ CRITICAL
**Severity**: High
**Location**: `src/parser.rs:1114-1146` - `parse_power` function
**Test Cases**: `bug_power_right_associativity`, `bug_power_chain`

**Description**:
The power operator `**` is implemented with left associativity instead of right associativity, breaking Python compatibility.

**Expected Behavior**:
```python
2**3**2  # Should be 2**(3**2) = 2**9 = 512
2**2**3  # Should be 2**(2**3) = 2**8 = 256
```

**Actual Behavior**:
```python
2**3**2  # Returns (2**3)**2 = 8**2 = 64 ❌
2**2**3  # Returns (2**2)**3 = 4**3 = 64 ❌
```

**Root Cause**:
The `parse_power` function uses a while loop that builds the AST left-to-right, making it left-associative. Python's `**` operator should be right-associative.

**Fix Needed**:
Change the power parsing to be right-recursive instead of iterative left-to-right.

---

### 2. Parentheses Ignored Around Negative Numbers in Power Expressions ⚠️ CRITICAL
**Severity**: High
**Location**: `src/parser.rs` - interaction between `parse_unary` and `parse_power`
**Test Case**: `bug_power_negative_base_parens`

**Description**:
Parentheses around negative numbers are being ignored when used with the power operator.

**Expected Behavior**:
```python
(-2)**2  # Should be 4
```

**Actual Behavior**:
```python
(-2)**2  # Returns -4 ❌
```

**Root Cause**:
The grammar has `unary = { unary_op* ~ power }`, which means unary operators are parsed *around* power expressions, not *within* them. The parser is treating `(-2)**2` as `-(2**2)` even with parentheses.

**Fix Needed**:
This appears to be a complex precedence issue in the grammar. The parenthesized expression should be treated as a primary expression before power is applied.

---

### 3. Try Expression Without Fallback Returns Error Message String ⚠️ CRITICAL
**Severity**: High
**Location**: `src/lower.rs` - `lower_expr_with_exception` function, specifically TryExpr handling
**Test Case**: `bug_try_expr_no_fallback_error`

**Description**:
When a try expression has no fallback (`expr?`), it should return `None` on error. Instead, it returns the error message as a string.

**Expected Behavior**:
```python
x = (1/0)?
print(x)  # Should print "None"
```

**Actual Behavior**:
```python
x = (1/0)?
print(x)  # Prints "division by zero" ❌
```

**Root Cause**:
The `__snail_compact_try` helper function likely has incorrect default fallback behavior when no fallback lambda is provided.

**Fix Needed**:
Review the Python helper function that handles compact try expressions to ensure it returns `None` when no fallback is provided and an exception occurs.

---

### 4. Power Operator Doesn't Support Negative Exponents ⚠️ MAJOR
**Severity**: Medium
**Location**: `src/snail.pest` - grammar rule for `power`
**Test Case**: `bug_power_negative_exponent`

**Description**:
The parser doesn't allow unary operators (like `-`) as the right-hand side of the power operator.

**Expected Behavior**:
```python
2**-1  # Should be 0.5
```

**Actual Behavior**:
```
Parse error: "expected primary" ❌
```

**Root Cause**:
The grammar defines `power = { primary ~ (pow_op ~ primary)* }`, which only allows primary expressions as exponents, not unary expressions.

**Fix Needed**:
Change the grammar to allow unary expressions as the right-hand side of power:
```pest
power = { primary ~ (pow_op ~ unary)* }
```

However, this must be done carefully to maintain right associativity.

---

### 5. Nested Try Expressions with $e Don't Work Correctly ⚠️ MAJOR
**Severity**: Medium
**Location**: `src/lower.rs` - exception variable handling in nested try expressions
**Test Case**: `bug_nested_try_exception_var`

**Description**:
When try expressions are nested, the `$e` variable doesn't properly scope to the innermost exception.

**Expected Behavior**:
```python
x = (1/0) ? ($e.args[0] ? 'inner')
# Inner $e should refer to the inner exception
```

**Actual Behavior**:
The nested $e handling doesn't work as expected (test fails).

**Root Cause**:
The lambda-based exception handling may not properly handle nested exceptions with the same `$e` variable name. The scoping may be leaking or shadowing incorrectly.

**Fix Needed**:
Review the exception variable passing in nested try expressions. May need unique variable names for each level of nesting.

---

## Test Coverage Summary

### Bugs Confirmed with Tests (Marked with `#[should_panic]`)
1. ✅ Power operator wrong associativity (2 test cases)
2. ✅ Parentheses ignored with power operator (1 test case)
3. ✅ Try expression without fallback returns error message (1 test case)
4. ✅ Negative exponent not supported (1 test case)
5. ✅ Nested try expressions with $e broken (1 test case)

### Edge Cases That Work Correctly (Tests Pass)
- Field index validation for negative/overflow values
- AWK mode field out of bounds handling
- AWK mode empty/whitespace lines
- Division/floor division/modulo by zero (proper errors)
- Empty list/dict comprehensions
- Slice operations (reversed bounds, negative indices)
- String slicing edge cases
- Unary minus/plus with power (precedence correct)
- Complex operator precedence
- AWK tab field splitting
- Try expressions with explicit fallbacks
- Empty dict/set literals
- Boolean operators (and, or, not)
- Chained comparisons
- Nested parentheses
- Power precedence with multiplication/addition
- Regex literals
- String escapes and raw strings
- Empty tuples and single-element tuples
- And 20+ more edge cases...

### Test Statistics
- **Total Test Cases**: 50
- **Passing Tests**: 45
- **Expected Failures (Bugs)**: 5
- **Test Coverage**: Comprehensive coverage of operator precedence, edge cases, AWK mode, try expressions, and basic language features

---

## Recommendations

1. **Priority 1**: Fix power operator associativity - this is a fundamental operator precedence issue
2. **Priority 1**: Fix parentheses handling with power operator - breaks basic mathematical expressions
3. **Priority 2**: Fix try expression fallback behavior - affects error handling semantics
4. **Priority 2**: Add support for negative exponents - common Python use case
5. **Priority 3**: Fix nested exception variable scoping - less common but important for complex error handling

---

## Testing Methodology

The bug hunting process involved:
1. Analysis of the codebase to identify complex/error-prone areas
2. Study of Python semantics for comparison
3. Creation of edge case tests based on operator precedence, expression nesting, and boundary conditions
4. Systematic testing of AWK mode, try expressions, operator combinations, and data structure edge cases
5. Confirmation of bugs through automated test execution

All bug tests are preserved in `tests/bug_hunting.rs` with clear documentation of expected vs actual behavior.
