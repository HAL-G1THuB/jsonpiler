# Types

## Primitive Types

- **Null**: Represents the absence of a value. The concept is similar to the Unit value.
- **Bool**: Represents a 8-bit boolean value, either `true` or `false`.
- **Int**: Represents a 64-bit integer number.
- **Float**: Represents a 64-bit floating-point number.
- **Str**: Represents a UTF-8 string enclosed in double quotes.

## Composite Types

- **Array**: Represents an ordered collection of values.

### Subtypes of Object

- **HashMap**: Represents a collection of key-value pairs.

```json
{ "key": "value" }
```

- **Block**: Represents an ordered sequence of instructions.

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
  "x": "Str" ,
  "return": "Str"
}
```
