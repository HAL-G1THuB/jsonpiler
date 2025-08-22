# Arithmetic operations

## abs

```json
{"abs": "Int"} -> "Int (Temporary Value)"
```

```text
abs(int)
```

Returns the absolute value of the given integer.

```json
{"abs": -5} => 5
```

## +

```json
{"+": ["Int", "Int", "..."]} -> "Int (Temporary Value)"
```

```text
int + int
+(int, int, int)
```

Returns the sum of all operands.

```json
{ "+": [ 1, 5, ["+", 4, 6]] } => 16
```

## -

```json
{ "-": ["Int", "..."] } -> "Int (Temporary Value)"
```

```text
int - int
-(int, int, int)
```

Subtracts all following operands from the first one and returns the result.
If given one argument, invert the sign.

```json
{"-": [30, 5, {"+": [4, 6]}]} => 15
```

## *

```json
{"*": ["Int", "Int", "..."]} -> "Int (Temporary Value)"
```

```text
int * int
*(int, int, int)
```

Returns the result of multiplying operands.

```json
{"*": [30, 5, {"+":[4, 6]}]} => 1500
```

## /

```json
{"/": ["Int", "Int", "..."]} -> "Int (Temporary Value)"
```

```text
int / int
/(int, int, int)
```

Returns the result of dividing the first operand by all following operands.
If the number to divide is zero, an error is generated at runtime or compile time.

```json
{"/": [30, 5, 6]} => 1
```

## %

```json
{"%": ["Int", "Int"]} -> "Int (Temporary Value)"
```

```text
int % int
%(int, int)
```

Returns the result of the remainder operation.

```json
{"%": [30, 7]} => 2
```
