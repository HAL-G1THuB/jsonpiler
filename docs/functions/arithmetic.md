# Arithmetic operations

## abs

```json
{"abs": {"expr": "Int"}} -> "VInt"
```

Returns the absolute value of the given integer.

```json
{"abs": -5} => 5
```

## +

```json
{"+": [{ "operand": "Int..." }, "..."]} -> "VInt"
```

Returns the sum of all operands.
If given zero arguments, it returns the identity element (0).

```json
{ "+": [ 1, 5, ["+", 4, 6]] } => 16
```

## -

```json
{ "-": [{ "operand": "Int..." }, "..."] } -> "VInt"
```

Subtracts all following operands from the first one and returns the result.
If given zero arguments, it returns the identity element (0).
If given one argument, invert the sign.

```json
{"-": [30, 5, {"+": [4, 6]}]} => 15
```

## *

```json
{"*": [{"operand": "Int..."}, "..."]} -> "VInt"
```

Returns the result of multiplying operands.
If given zero arguments, it returns the identity element (1).

```json
{"*": [30, 5, {"+":[4, 6]}]} => 1500
```

## /

```json
{"/": [{"operand": "Int"}, {"operand": "Int"}, "..."]} -> "VInt"
```

Returns the result of dividing the first operand by all following operands.
If less than 1 arguments are given, an error occurs.
If the number to divide is zero, an error is generated at runtime or compile time.

```json
{"/": [30, 5, 6]} => 1
```

## %

```json
{"%": [{"operand": "Int"}, {"operand": "Int"}]} -> "VInt"
```

Returns the result of the remainder operation.

```json
{"%": [30, 7]} => 2
```
