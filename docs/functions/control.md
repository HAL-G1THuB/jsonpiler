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

Registers a user-defined function with the following parameters:

- The first argument specifies the function’s name.
- The second argument provides the type annotation for the function’s parameter.
- The third argument specifies the return type annotation.
- The fourth argument is the function body, which is evaluated when the function is called.

Up to 16 arguments can be defined for a single function.
The `define` keyword also introduces a new scope.

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
          "`then` branch."
        ]
      }
    ],
    [{"==": [1, 2]},
      {"message": [
          "1 == 2🤔",
          "`else if` branch."
        ]
      }
    ],
    [true,
      {"message": [
          "1 == ?🤣",
          "`else` branch."
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
{
  "=": ["i", 0],
  "while": [
  {"<": [{"$": "i"}, 5]},
  {"scope": [
    {"message": ["Loop", {"$": "i"}]},
    {"=": ["i", {"+": [{"$": "i"}, 1]}]}
  ]}
]}
```
