# Specification

Language specification of the Jsompiler.

## Syntax

The syntax is based on the JSON specification.  
See: [JSON specification](https://www.rfc-editor.org/info/rfc8259)

## Evaluation

A Jsompiler program is represented as a single JSON value.

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
