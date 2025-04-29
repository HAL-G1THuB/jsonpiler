# Specification

Specification of the Jsonpiler language.

## Syntax

The syntax is based on the JSON specification.  
See: [JSON specification](https://www.rfc-editor.org/info/rfc8259)

## Evaluation

A jsonpiler program is represented as a single JSON value.

Each JSON value is evaluated independently, except in the case of arrays ([]) and objects ({}).

Lists are evaluated as follows:

- The first element of the list is treated as the built-in function name or user-defined function name.
- This element must be a string (representing a function name) or a lambda expression.

```json
["lambda", [], ["+", 3, 1]]
```

- The remaining elements are passed as arguments to the function.

Objects are evaluated as follows:

- The object preserves insertion order.
- The values of the object's properties are evaluated in that order.

## Exits

The exit code returned by `jsonpiler::functions::run` is wrapped using modulo 256,
resulting in a value between 0 and 255.

## Encoding

This program supports UTF-8.
