# Control frow

## define

```jspl
define(
  name: Str (Literal),
  params: TypeAnnotations,
  return_type: Str (Literal),
  body: Block
) -> "Null"
```

Registers a user-defined function with the following parameters:

- The first argument specifies the function’s name.
- The second argument provides the type annotation for the function’s parameter.
- The third argument specifies the return type annotation.
- The fourth argument is the function body, which is evaluated when the function is called.

The `define` keyword also introduces a new scope.

```jspl
define(by_two, {n: Int}, Int, $n + $n)
by_two(2) => 4
```

**Types that can be assigned to arguments**:

- `Int`
- `Str`
- `Bool`
- `Null`
- `Float`

**Types that can be returned by the function**:

- `Int`
- `Str`
- `Bool`
- `Null`
- `Float`

## if

```jspl
if([condition: Bool, then: Block], ...) -> "Null"
```

Evaluates each condition in order. If a condition evaluates to `true`, the corresponding `then` expression is executed.
Regardless of which branch is taken, the overall result is always `null`.

```jspl
if(
[1 == 1, message("1 == 1✨", "`then` branch.")]
[1 == 2, message(1 == 2🤔", "`else if` branch.")]
[true, message("1 == ?🤣", "`else` branch.")]
)
  => null
```

## while

```jspl
while(condition: Bool, body: Block) -> "Null"
```

Executes the `body` repeatedly as long as the `condition` evaluates to `true`.
Returns `null`.

```jspl
i = 0
while($i < 5
  scope({
    message("Loop", "");
    i += 1
  })
)
```

## include

```jspl
include(path: Str (Literal), functions: Str (Literal), ...) -> "Null"
```

Executes the specified file and includes the specified function in the namespace.
The path is relative to the current file.
This function does not affect existing variables.
If the same file is included more than once, no new code is generated,
and functions can be included from existing code.
including a function from a different file with the same name causes a redefinition error.
An error occurs if the file does not contain the specified function.

```jspl
include("my_library.jspl", my_func)
my_func()
```

## ret

```jspl
ret(Any) -> Null
```

Terminates execution of the function and returns the given value.
`return` may only be used within a function defined by `define`.
A `return` may only be written at the end of any block.

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
