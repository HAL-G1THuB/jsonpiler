# Arithmetic operations

## abs

```jspl
abs(Int) -> Int
abs(Float) -> Float
```

Returns the absolute value of `Int` or `Float`.
If the value is `0x8000000000000000` (the smallest 64-bit signed integer),
the result is unchanged due to two's complement representation.

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
If the operands are strings, they are concatenated.

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

Subtracts each subsequent operand from the first.
With one argument, the sign is inverted.

```jspl
30 - 5 - { 4 + 6 }
  => 15
```

## \*

```jspl
Int * Int... -> Int
Float * Float... -> Float
```

Multiplies all operands.

```jspl
30 * 5 * { 4 + 6 } => 1500
```

## /

```jspl
Int / Int... -> Int
Float / Float... -> Float
```

Divides the first operand by each subsequent operand.
If any divisor is zero, an error is generated at runtime or compile time.

```jspl
30 / 5 / 6 => 1
```

## %

```jspl
Int % Int -> Int
```

Computes the remainder of the first operand divided by the second.

```jspl
30 % 7 => 2
```

## Int

```jspl
Int(Float) -> Int
```

Converts `Float` to `Int` by discarding the fractional part.

```jspl
Int(1.5) => 1
```

## Float

```jspl
Float(Int) -> Float
```

Converts `Int` to `Float` by adding .0

```jspl
Float(1) => 1.0
```

## random

```jspl
random() -> Int
```

Returns a pseudo-random 64-bit integer.
Not suitable for cryptographic purposes.

```jspl
{"random": []} => 1234567890
```

## <<

```jspl
Int << Int -> Int
```

Performs a left bitwise shift.

```jspl
1 << 3 => 8
```

## >>

```jspl
Int >> Int -> Int
```

Performs a right bitwise shift.

## sqrt

```jspl
sqrt(Float) -> Float
```

Returns the square root of `Float`.

```jspl
sqrt(2.0) => 1.414...
```
