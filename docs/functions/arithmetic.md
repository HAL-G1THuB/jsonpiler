# Arithmetic operations

## abs

```jspl
abs(Int) -> Int
```

Returns the absolute value of the given integer.
If the given integer is `0x8000000000000000` (the smallest 64-bit signed integer), the result is `0x8000000000000000` itself due to the nature of two's complement representation.

```jspl
abs(-5)
  => 5
```

## +

```jspl
+(Float or Int, Float or Int, ...) -> Float or Int (Temporary Value or Literal)
```

Returns the sum of all operands.

```jspl
1 + 5 + 4 + 6
  => 16
```

## -

```jspl
-(Float or Int, ...) -> Float or Int (Temporary Value or Literal)
```

Subtracts each subsequent operand from the first and returns the result.
If given one argument, invert the sign.

```jspl
30 - 5 - { 4 + 6 }
  => 15
```

## *

```jspl
*(Float or Int, Float or Int, ...) -> Float or Int (Temporary Value or Literal)
```

Returns the result of multiplying operands.

```jspl
30 * 5 * { 4 + 6 } => 1500
```

## /

```jspl
/(Float or Int, Float or Int, ...) -> Float or Int (Temporary Value or Literal)
```

Returns the result of dividing the first operand by all following operands.
If the number to divide is zero, an error is generated at runtime or compile time.

```jspl
30 / 5 / 6 => 1
```

## %

```jspl
%(Int, Int) -> Int (Temporary Value or Literal)
```

Returns the result of the remainder operation.

```jspl
30 % 7 => 2
```

## Int

```jspl
Int(Float) -> Int
```

Returns the integer part of the given float.

```jspl
Int(1.5) => 1
```

## Float

```jspl
Float(Int) -> Float
```

Converts an integer to a float by adding .0

```jspl
Float(1) => 1.0
```

## random

```jspl
random() -> Int
```

Returns a pseudo-random 64-bit integer.
Not suitable for cryptography.

```jspl
{"random": []} => 1234567890
```
