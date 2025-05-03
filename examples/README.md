# Examples

[Display a message box](https://github.com/HAL-G1THuB/jsonpiler/tree/main/examples/message_box.json)

```json
{ "message": ["Hello", "This is a message box!"] }
```

[Arithmetic operations](https://github.com/HAL-G1THuB/jsonpiler/tree/main/examples/arithmetic.json)

```json
{
  "'": "This program will return the number 16.",
  "-": [{ "*": [5, { "+": [2, 3] }] }, 9]
}
```

[Scope](https://github.com/HAL-G1THuB/jsonpiler/tree/main/examples/scope.json)

```json
{
  "'": "This program will return the number 123, not 100.",
  "=": ["x", 123],
  "scope": ["=", "x", 100],
  "$": "x"
}
```
