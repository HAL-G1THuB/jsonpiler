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
{"not": "Bool"} -> "Bool (Temporary Value)"
```

```jspl
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

```jspl
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

```jspl
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

```jspl
bool xor bool
```

Returns the logical OR of the given booleans.

```json
{"xor": [true, false]} => true
```
