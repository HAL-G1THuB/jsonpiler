# Arithmetic operations

## abs

```jspl
abs(Int) -> Int
abs(Float) -> Float
```

Returns the absolute value of the given integer or float.
If the given integer is `0x8000000000000000` (the smallest 64-bit signed integer), the result is `0x8000000000000000` itself due to the nature of two's complement representation.

```jspl
abs(-5) => 5
```

## +

```jspl
Int + Int... -> Int
Float + Float... -> Float
Str + Str... -> Str
```

Returns the sum of all operands.
if all operands are strings, the result is the concatenation of all strings.

```jspl
1 + 5 + 4 + 6 => 16
"Hello" + "World" => "HelloWorld"
```

## -

```jspl
-(Int)
-(Float)
Int - Int... -> Int
Float - Float... -> Float
```

Subtracts each subsequent operand from the first and returns the result.
If given one argument, invert the sign.

```jspl
30 - 5 - { 4 + 6 }
  => 15
```

## \*

```jspl
Int * Int... -> Int
Float * Float... -> Float
```

Returns the result of multiplying operands.

```jspl
30 * 5 * { 4 + 6 } => 1500
```

## /

```jspl
Int / Int... -> Int
Float / Float... -> Float
```

Returns the result of dividing the first operand by all following operands.
If the divisor is zero, an error is generated at runtime or compile time.

```jspl
30 / 5 / 6 => 1
```

## %

```jspl
Int % Int -> Int
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

## <<

```jspl
Int << Int -> Int
```

Returns the result of left bitwise shift.

```jspl
1 << 3 => 8
```

## >>

```jspl
Int >> Int -> Int
```

Returns the result of right bitwise shift.

## sqrt

```jspl
sqrt(Float) -> Float
```

Returns the square root of the given float.

```jspl
sqrt(2.0) => 1.414...
```
