# Compound Assignment Operators

## +=

```jspl
Ident += Float -> Null
Ident += Int -> Null
Ident += Str -> Null
```

Adds the value to the variable and assigns the result to the variable.
If value is a string, it concatenates the string to the variable.

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

Subtracts the value from the variable and assigns the result to the variable.

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

Multiplies the variable by the value and assigns the result to the variable.

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

Divides the variable by the value and assigns the result to the variable.
If the value is zero, an error is generated at runtime or compile time.

```jspl

let(i = 30)
i /= 5 => null
i => 6
```
