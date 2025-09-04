# Boolean logic

## assert

```json
{"assert": ["Bool", "String"]} -> "Null"
```

```jspl
assert(bool)
```

If the given boolean is `false`, an error is generated at runtime.

```json
{"assert": [false, "Assertion failed"]} ... message("", "Assertion failed") and terminate
```

## not

```json
{"not": "Bool or Int"} -> "Bool or Int (Temporary Value)"
```

```jspl
not(bool)
```

Returns the logical NOT of the given boolean or integer.

```json
{"not": true} => false
```

## and

```json
{"and": ["Bool or Int", "Bool or Int", "..."]} -> "Bool or Int (Temporary Value)"
```

```jspl
bool and bool
```

Returns the logical AND of the given booleans or integers.

```json
{"and": [true, false]} => false
```

## or

```json
{"or": ["Bool or Int", "Bool or Int", "..."]} -> "Bool or Int (Temporary Value)"
```

```jspl
bool or bool
```

Returns the logical OR of the given booleans or integers.

```json
{"or": [true, false]} => true
```

## xor

```json
{"xor": ["Bool or Int", "Bool or Int", "..."]} -> "Bool or Int (Temporary Value)"
```

```jspl
bool xor bool
```

Returns the logical OR of the given booleans or integers.

```json
{"xor": [true, false]} => true
```
