# Control frow

## define

```json
{
  "define": [
    {"name": "String (Literal)"},
    {"params": "TypeAnnotations"},
    {"return_type": "String (Literal)"},
    {"body": "Sequence"}
  ]
} -> "Null"
```

```text
define(name, params, return_type, body)
```

Registers a user-defined function.
The first argument is the name of the function.
The second argument is the type annotation of the argument.
The third argument is the type annotation of the return value.
The fourth argument is the function body and is evaluated when the function is called.
`define` introduces a new scope.

```json
{"define": ["*2", {"n": "Int"}, {"+": [{"$": "n"}, {"$": "n"}]}]}
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

```text
if([condition, then], ...)
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

## while

```json
{"while": [{"condition": "Bool"}, {"body": "Sequence"}]} -> "Null"
```

```text
while(condition, body)
```

Executes the `body` repeatedly as long as the `condition` evaluates to `true`.
Returns `null`.

```json
{"while": [
  {"<": [{"$": "i"}, 5]},
  {"scope": [
    {"message": ["Loop", {"$": "i"}]},
    {"=": ["i", {"+": [{"$": "i"}, 1]}]}
  ]}
]}
