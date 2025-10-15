# Control frow

## define

```json
{
  "define": [
    {"name": "String (Literal)"},
    {"params": "TypeAnnotations"},
    {"return_type": "String (Literal)"},
    {"body": "Block"}
  ]
} -> "Null"
```

```jspl
define(name, params, return_type, body)
```

Registers a user-defined function with the following parameters:

- The first argument specifies the function’s name.
- The second argument provides the type annotation for the function’s parameter.
- The third argument specifies the return type annotation.
- The fourth argument is the function body, which is evaluated when the function is called.

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
  [{"condition": "Bool"}, {"then": "Block"}], "..."
  ]
} -> "Null"
```

```jspl
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
{"while": [{"condition": "Bool"}, {"body": "Block"}]} -> "Null"
```

```jspl
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

## include

```json
{"include": [{"path": "String (Literal)"}, {"functions": "String (Literal)"}, "..."]} -> "Null"
```

```jspl
include("path/to/file.jspl", fib)
```

Executes the specified file and includes the specified function in the namespace.
The path is relative to the current file.
This function does not affect existing variables.
If the same file is included more than once, no new code is generated,
and functions can be included from existing code.
including a function from a different file with the same name causes a redefinition error.
An error occurs if the file does not contain the specified function.

```json
{"include": "my_library.jspl", "my_func"}
{"my_func": ["arg"]}
```

## ret

```json
{"ret": "Any"} -> "Null"
```

```jspl
ret(value)
```

Terminates execution of the function and returns the given value.
`return` may only be used within a function defined by `define`.
A `return` may only be written at the end of any block.

## break

```json
{"break": []} -> "Null"
```

```jspl
break()
```

Terminates the innermost `while` loop.
`break` may only be used within a `while` loop.

## continue

```json
{"continue": []} -> "Null"
```

```jspl
continue()
```

Terminates the current iteration of the innermost `while` loop.
`continue` may only be used within a `while` loop.
