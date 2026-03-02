# Boolean logic

## assert

```jspl
assert(Bool, Str) -> Null
```

If the given boolean is `false`, an error is generated at runtime.

```jspl
assert(false, "Assertion failed")
...
RuntimeError:
  AssertionError:
   Assertion failed
Error at assert.jspl line: 1 column: 6
Error position: 
assert(false, "Assertion failed")
       ^^^^^
```

## not

```jspl
not(Bool or Int") -> Bool or Int
```

Returns the NOT of the given boolean or integer.

```jspl
not(true) => false
```

## and

```jspl
and(Bool or Int, Bool or Int, ...) -> Bool or Int
```

Returns the AND of the given booleans or integers.

```jspl
true and false => false
```

## or

```jspl
or(Bool or Int, Bool or Int, ...) -> Bool or Int
```

Returns the OR of the given booleans or integers.

```jspl
true or false => true
```

## xor

```jspl
xor(Bool or Int, Bool or Int, ...) -> Bool or Int
```

Returns the logical OR of the given booleans or integers.

```jspl
true xor false => true
```
