# Evaluation and list

## '

```json
{"'": {"expr": "Any"}} -> {"unevaluated_expr": "Any"}
```

Returns the expression without evaluating it.
Can also be used as a comment.

## eval

```json
{"eval": {"expr": "Any"}} -> {"evaluated_expr": "Any"}
```

Evaluates the given expression and returns the result.

## list

```json
{"list": [{"expr": "Any"}, "..."]} -> "Array (Literal)"
```

The list function returns its evaluated arguments as an Array (Literal).

```json
{"list": {"+": [3, 5]}} => [8]
```

## value

```json
{"value": {"value": "Any"}} -> "Any"
```

Returns the given value.

```json
{"value": 8} => 8
```
