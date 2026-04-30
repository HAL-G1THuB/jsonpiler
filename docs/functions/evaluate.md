# Evaluation and list

## list

```jspl
list(Any, ...) -> Array (Literal)
```

Returns its arguments as an `Array` (literal).

```jspl
list(3 + 5, 0) => [8, 0]
```

## value

```jspl
value(Any) -> Any
```

Returns the given value.

```jspl
value(8) => 8
```

## main

```jspl
main(Any) -> Any
```

Evaluates the `Block` when the file is executed as the main program.
