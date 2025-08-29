# Comparison

## !=

```json
{"!=": ["Int", "..."]} -> "Bool (Temporary Value)"
```

```jspl
bool != bool
```

Returns `true` if all arguments are not equal, `false` otherwise.

```json
{"!=": [1, 1]} => false
```

## ==

```json
{"==": ["Int", "..."]} -> "Bool (Temporary Value)"
```

```jspl
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

```jspl
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

```jspl
int <= int
```

Returns `true` if the arguments are in increasing order, `false` otherwise.

```json
{"<=": [1, 2]} => true
```

## >=

```json
{">=": ["Int", "Int"]} -> "Bool (Temporary Value)"
```

```jspl
int >= int
```

Returns `true` if the arguments are in decreasing order, `false` otherwise.

```json
{">=": [2, 1]} => true
```

## >

```json
{">": ["Int", "..."]} -> "Bool (Temporary Value)"
```

```jspl
int > int
```

Returns `true` if the arguments are in strictly decreasing order, `false` otherwise.

```json
{">": [2, 1]} => true
```
