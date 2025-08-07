# Boolean logic

## not

```json
{"not": {"expr": "Bool"}} -> "VBool"
```

Returns the logical NOT of the given boolean.

```json
{"not": true} => false
```

## and

```json
{"and": [{"arg": "Bool"}, "..."]} -> "VBool"
```

Returns the logical AND of the given booleans.

```json
{"and": [true, false]} => false
```

## or

```json
{"or": [{"arg": "Bool"}, "..."]} -> "VBool"
```

Returns the logical OR of the given booleans.

```json
{"or": [true, false]} => true
```

## xor

```json
{"xor": [{"arg": "Bool"}, "..."]} -> "VBool"
```

Returns the logical OR of the given booleans.

```json
{"xor": [true, false]} => true
```
