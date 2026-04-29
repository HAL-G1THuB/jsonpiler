# String

## len

```jspl
len(Str) -> Int
```

Returns the number of characters.
This is based on characters, not bytes.

```jspl
len("Hello, World!") => 13
```

## Str

```jspl
Str(Int) -> Str
```

Converts the given value into a string.

```jspl
Str(123) => "123"
```

## slice

```jspl
slice(Str, Int) -> Str
slice(Str, Int, Int) -> Str
```

Returns a sub-string of the given UTF-8 string from the `start` character index to the `end` character index (exclusive).
The indices are based on characters, not bytes.
If the indices are out of bounds, return `""`.

```jspl
slice("Jsonpiler", 0, 4) => "Json"
```

Negative indices can be used to specify positions from the end of the string
For example, `-1` refers to the last character.

```jspl
slice("Jsonpiler", -5, -1) => "pile"
```

If the `end` index is omitted, the substring from `start` to the end of the string is returned.

```jspl
slice("Jsonpiler", 0) => "Jsonpiler"

slice("Jsonpiler", 0, len("Jsonpiler")) => "Jsonpiler"
```
