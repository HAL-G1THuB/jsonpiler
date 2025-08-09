# Specification

Specification of the Jsonpiler language.

## Syntax

The syntax is based on the JSON specification.  
See: [JSON specification](https://www.rfc-editor.org/info/rfc8259)

## Types

### Primitive Types

- **Null**: Represents the absence of a value.
- **Bool**: Represents a 8-bit boolean value, either `true` or `false`.
- **Int**: Represents a 64-bit integer number.
- **Float**: Represents a 64-bit floating-point number.
- **String**: Represents a sequence of characters enclosed in double quotes.

### Composite Types

- **Array**: Represents an ordered collection of values.

#### Object

- **HashMap**: Represents a collection of key-value pairs.

```json
{ "key": "value" }
```

- **Sequence**: Represents an ordered sequence of instructions.

```json
{
  "message": ["123", "456"],
  "+": [1, 2]
}
```

- **TypeAnnotations**: Represents a type annotation for a variable or function.

```json
{
  "i": "Int",
  "x": "String" ,
  "return": "String"
}
```

### Function Type

- **Function**: Represents a function that can be called with arguments.

## Evaluation

A Jsonpiler program is represented as a single JSON value.

Each JSON value is evaluated independently, except in the case of arrays (`[]`) and objects (`{}`).

### Arrays

- Each element in the array is evaluated sequentially.
- The result is a new array containing all evaluated elements, preserving their order.

```json
[1, {"+": [2, 3]}, "text"]
// => [1, 5, "text"]
```

### Objects

- Each key in the object is interpreted as a function name.
- Each value is evaluated and passed as the argument to the function.
- If the key matches a registered built-in function, that is used; otherwise, a user-defined function is looked up.
- Multiple key-function entries are supported and evaluated in insertion order.
- The result of the last function call is returned as the final value.

```json
{
  "message": ["123", "456"],
  "+": [1, 2]
}
// => evaluates "message" then "+", returns result of "+"
```

## Exits

The exit code returned by `jsonpiler::functions::run` is wrapped using modulo 256,  
resulting in a value between 0 and 255.

## Encoding

This program supports UTF-8.
