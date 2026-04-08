# Specification

Specification of the Jsonpiler language.

## Evaluation

A Jsonpiler program is represented as a single JSON value.

Each JSON value is evaluated independently, except in the case of arrays (`[]`) and objects (`{}`).

### Arrays

- Each element in the array is evaluated sequentially.
- The result is a new array containing all evaluated elements, preserving their order.

```json
[1, { "+": [2, 3] }, "text"]
// => [1, 5, "text"]
```

### Objects

- Each key in the object is interpreted as a function name.
- Each value is evaluated and passed as the argument to the function.
- If the key matches a registered built-in function, that is used; otherwise, a user-defined function is looked up.
- Multiple key-function entries are supported and evaluated in insertion order.
- The result of the last function call is returned as the final value.
- An empty `Block` returns Null.

```json
{
  "message": ["123", "456"],
  "+": [1, 2]
}
// => evaluates "message" then "+", returns result of "+"
```

## Exits

If the entire program evaluates to Int, it returns it as an exit code.
The exit code returned by jsonpiler is a 32-bit signed integer that is the exit code of the generated .exe file.
If a compilation error occurs or the executable file is not generated, the exit code is 1.

## Encoding

This program supports UTF-8.

## Operator Precedence List

Higher precedence means higher binding priority

| Precedence | Operator                         |
| ---------- | -------------------------------- |
| 0          | `=`, `+=`, `-=`, `*=`, `/=`      |
| 1          | `or`                             |
| 2          | `xor`                            |
| 3          | `and`                            |
| 4          | `<`, `<=`, `>`, `>=`, `==`, `!=` |
| 5          | `+`, `-`                         |
| 6          | `*`, `/`, `%`                    |
