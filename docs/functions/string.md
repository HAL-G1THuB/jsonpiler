# String

## concat

```json
{"concat": ["String (Literal)", "String (Literal)", "..."]} -> "String (Literal)"
```

```jspl
concat(string, string...)
```

Returns the result of concatenating all string literals.

```json
{"concat": ["Hello", "World"]} => "HelloWorld"
```

## len

```json
{"len": "String (Literal)"} -> "String (Literal)"
```

```jspl
len(string)
```

Returns the length of the string.

```json
{"len": "Hello, World!"} => 13
```
