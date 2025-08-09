# Control frow

## lambda

```json
{"lambda": [{"params": "TypeAnnotations"}, {"body": "Sequence"}]} -> "Function"
```

Creates a function.  
The first argument specifies the parameters as an object of type annotations;  
the remaining arguments form the function body and are evaluated when the function is called.
`lambda` introduces a new scope.

```json
{"lambda": [[], {"+": [4, 6]}, 1]}
```

**Types that can be assigned to arguments**:

- `Int`
- `String`
- `Bool`
- `Null`
- `Float`

**Types that can be returned by the function**:

- `Int`
- `String`
- `Bool`
- `Null`
- `Float`

## if

```json
{"if": [
  [{"condition": "Bool"}, {"then": "Sequence"}], "..."
  ]
} -> "Null"
```

Evaluates each condition in order. If a condition evaluates to `true`, the corresponding `then` expression is executed.
Regardless of which branch is taken, the overall result is always `null`.

```json
{"if": [
    [{"==": [1, 1]},
      {"message": [
          "1 == 1✨",
          "Placing `true` in the first condition acts as the `then` branch."
        ]
      }
    ],
    [{"==": [1, 2]},
      {"message": [
          "1 == 2🤔",
          "Placing `true` in this condition acts as the `else if` branch."
        ]
      }
    ],
    [true,
      {"message": [
          "1 == ?🤣",
          "Placing `true` in the condition here acts as the `else` branch."
        ]
      }
    ]
  ]
}
 => null
```
