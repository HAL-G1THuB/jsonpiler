# Evaluation and list

## '

```jspl
'(Any) -> unevaluated: Any
```

Returns the expression without evaluating it.
Can also be used as a comment.

## eval

```jspl
eval(Any) -> evaluated: Any
```

Evaluates the given expression and returns the result.

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
