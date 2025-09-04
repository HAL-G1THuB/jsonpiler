# Arithmetic operations

## abs

```json
{"abs": "Int"} -> "Int (Temporary Value)"
```

```jspl
abs(int)
```

Returns the absolute value of the given integer.
If the given integer is `0x8000000000000000` (the smallest 64-bit signed integer), the result is `0x8000000000000000` itself due to the nature of two's complement representation.

```json
{"abs": -5} => 5
```

## +

```json
{"+": ["Float or Int", "Float or Int", "..."]} -> "Float or Int (Temporary Value or Literal)"
```

```jspl
int + int
+(int, int, int)
```

Returns the sum of all operands.

```json
{ "+": [ 1, 5, ["+", 4, 6]] } => 16
```

## -

```json
{ "-": ["Float or Int", "..."] } -> "Float or Int (Temporary Value or Literal)"
```

```jspl
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
{"*": ["Float or Int", "Float or Int", "..."]} -> "Float or Int (Temporary Value or Literal)"
```

```jspl
int * int
*(int, int, int)
```

Returns the result of multiplying operands.

```json
{"*": [30, 5, {"+":[4, 6]}]} => 1500
```

## /

```json
{"/": ["Float or Int", "Float or Int", "..."]} -> "Float or Int (Temporary Value or Literal)"
```

```jspl
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
{"%": ["Int", "Int"]} -> "Int (Temporary Value or Literal)"
```

```jspl
int % int
%(int, int)
```

Returns the result of the remainder operation.

```json
{"%": [30, 7]} => 2
```

## Int

```json
{"Int": "Float or Int"} -> "Float or Int (Temporary Value)"
```

```jspl
Int(float)
```

Returns the integer part of the given float.

```json
{"Int": 1.5} => 1
```

## random

```json
{"random": []} -> "Int (Temporary Value)"
```

```jspl
random()
```

Returns a pseudo-random 64-bit integer.
Not suitable for cryptography.

```json
{"random": []} => 1234567890
```
