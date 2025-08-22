# Boolean logic

## not

```json
{"not": "Bool"} -> "Bool (Temporary Value)"
```

```text
not(bool)
```

Returns the logical NOT of the given boolean.

```json
{"not": true} => false
```

## and

```json
{"and": ["Bool", "Bool", "..."]} -> "Bool (Temporary Value)"
```

```text
bool and bool
```

Returns the logical AND of the given booleans.

```json
{"and": [true, false]} => false
```

## or

```json
{"or": ["Bool", "Bool", "..."]} -> "Bool (Temporary Value)"
```

```text
bool or bool
```

Returns the logical OR of the given booleans.

```json
{"or": [true, false]} => true
```

## xor

```json
{"xor": ["Bool", "Bool", "..."]} -> "Bool (Temporary Value)"
```

```text
bool xor bool
```

Returns the logical OR of the given booleans.

```json
{"xor": [true, false]} => true
```

## ==

```json
{"==": ["Int", "..."]} -> "Bool (Temporary Value)"
```

```text
bool == bool
```

Returns `true` if all arguments are equal, `false` otherwise.

```json
{"==": [1, 1]} => true
```

## <

```json
{"<": ["Int", "..."]} -> "Bool (Temporary Value)"
```

```text
int < int
```

Returns `true` if the arguments are in strictly increasing order, `false` otherwise.

```json
{"<": [1, 2]} => true
```

## <=

```json
{"<=": ["Int", "Int"]} -> "Bool (Temporary Value)"
```

```text
int <= int
```

Returns `true` if the arguments are in increasing order, `false` otherwise.

```json
{"<=": [1, 2]} => true
```
