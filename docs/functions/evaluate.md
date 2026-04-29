# Evaluation and list

## list

```jspl
list(Any, ...) -> Array (Literal)
```

The list function returns its evaluated arguments as an Array (Literal).

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

Evaluate the `Block` when the file is executed as the main program.
