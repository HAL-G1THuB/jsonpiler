# Control flow

## define

```jspl
define(
  name: Ident,
  params: TypeAnnotations,
  return_type: Ident,
  body: Any
) -> "Null"
```

Registers a user-defined function with the following parameters:

- name: the function name.
- params: the parameter type.
- return_type: the return type.
- body: the function body.

The `define` keyword introduces a new scope.

```jspl
define(by_two, { n: Int }, Int, n + n)
by_two(2) => 4
```

**Types not allowed as arguments or return values**:

- `Array`
- `Object`

## if

```jspl
if([Bool, Any]...) -> Null
if(Bool, Any) -> Null
```

Evaluates each condition in order.
If a condition is `true`,
the corresponding `then` expression is evaluated.
Returns `null`.

```jspl
if(
  [1 == 1, message("1 == 1✨", "`then` branch.")]
  [1 == 2, message("1 == 2🤔", "`else if` branch.")]
  [true, message("1 == ?🤣", "`else` branch.")]
)
  => null
```

## while

```jspl
while(Bool, Block) -> Null
```

Evaluates the `body` repeatedly while the `condition` is `true`.
Returns `null`.

```jspl
i = 0
while(i < 5,
{
  message("Loop", "");
  i += 1
}
)
```

## import

```jspl
import(path: Str (Literal), functions: Ident, ...) -> Null
```

Evaluates the specified file at startup and adds the specified function to the namespace.
The path is relative to the current file.
Does not affect existing variables.
If the same file is imported more than once, no new code is generated, and functions are reused.
Importing a function with the same name from a different file causes a redefinition error.
An error occurs if the specified function is not found in the file or not exported.

```jspl
import("my_library.jspl", my_func)
my_func()
```

## export

```jspl
export(Ident, ...) -> Null
```

Exports the specified functions for import by other files.
An error occurs if a specified function is not defined in the current file.

## ret

```jspl
ret(Any) -> Null
```

Terminates the function and returns the given value.
`ret` may only be used within a function defined by `define`.

## break

```jspl
break() -> Null
```

Terminates the innermost `while` loop.
`break` may only be used within a `while` loop.

## continue

```jspl
continue() -> Null
```

Terminates the current iteration of the innermost `while` loop.
`continue` may only be used within a `while` loop.
