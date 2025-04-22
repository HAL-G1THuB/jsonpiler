# Specification

Language specification of the jsonpiler.

## Syntax

The syntax is based on the JSON specification.  
See: [JSON specification](https://www.rfc-editor.org/info/rfc8259)

## Evaluation

A jsonpiler program is represented as a single JSON value.

Each JSON value is evaluated independently, except for lists (`[]`) and objects (`{}`).

Lists are evaluated as follows:

- The first element of the list is treated as the function name.
- This element must be either a string or a lambda expression.

```json
["lambda", [], ["+", 3, 1]]
```

- The remaining elements are passed as arguments to that function.

Objects are evaluated as follows:

- The object preserves insertion order.
- The values of the object's properties are evaluated in that order.

## Exits

The Exit Code returned by `jsonpiler::functions::run` is the actual exit code wrapped to a value between 0 and 255 using modulo 256.

## Encoding

This program supports UTF-8.
