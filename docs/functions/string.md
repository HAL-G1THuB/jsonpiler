# String

## concat

```json
{"concat": ["String (Literal)", "String (Literal)", "..."]} -> "String (Literal)"
```

```text
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

```text
len(string)
```

Returns the length of the string.

```json
{"len": "Hello, World!"} => 13
```
