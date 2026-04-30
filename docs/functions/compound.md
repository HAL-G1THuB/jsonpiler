# Compound Assignment Operators

## +=

```jspl
Ident += Float -> Null
Ident += Int -> Null
Ident += Str -> Null
```

Adds a value to a variable and assigns the result.
If value is a string, it is concatenated to the variable.

```jspl
let(i = 5)
i += 3 => null
i => 8
```

## -=

```jspl
Ident -= Float -> Null
Ident -= Int -> Null
```

Subtracts a value from a variable and assigns the result.

```jspl
let(i = 5)
i -= 3 => null
i => 2
```

## \*=

```jspl
Ident *= Float -> Null
Ident *= Int -> Null
```

Multiplies a variable by a value and assigns the result.

```jspl
let(i = 5)
i *= 3 => null
i => 15
```

## /=

```jspl
Ident /= Float -> Null
Ident /= Int -> Null
```

Divides a variable by a value and assigns the result.
If the value is zero, an error is generated at runtime or compile time.

```jspl

let(i = 30)
i /= 5 => null
i => 6
```
