# String

## concat

```jspl
concat(Str (Literal), ...)
```

Returns the result of concatenating all string literals.

```jspl
concat("Hello", "World") => "HelloWorld"
```

## len

```jspl
len(Str) -> Str
```

Returns the length of the string.

```jspl
len(Hello, World!) => 13
```

## Str

```jspl
Str(Int) -> Str
```

Converts the given value into a string.

```jspl
Str(123) => "123"
```
