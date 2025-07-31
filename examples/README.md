# Examples

[Display a message box](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples/message_box.json)

```json
{ "message": ["Hello", "This is a message box!"] }
```

[Arithmetic operations](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples/arithmetic.json)

```json
{
  "'": "This program will return the number 8.",
  "/": [{ "-": [{ "*": [5, { "+": [2, 3] }] }, 9] }, 2]
}
```

[Scope](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples/scope.json)

```json
{
  "'": "This program will return the number 123, not 100.",
  "=": ["x", 123],
  "scope": ["=", "x", 100],
  "$": "x"
}
```

[Squaring a number with a global variable](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples/square_global.json)

```json
{
  "global": ["n", 5],
  "=": [
    "square_n",
    {
      "'": "This function returns 25.",
      "lambda": [
        [],
        {
          "*": [{ "$": "n" }, { "$": "n" }]
        }
      ]
    }
  ],
  "square_n": []
}
```
