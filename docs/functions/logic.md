# Boolean logic

## assert

```jspl
assert(Bool, Str) -> Null
```

If `Bool` is `false`, an error is generated at runtime.

```jspl
assert(false, "Assertion failed")

=>

╭- RuntimeError --------------
| AssertionError:
|   Assertion failed
|-----------------------------
| input.jspl:1:8
|-----------------------------
| assert(false, "Assertion failed")
|        ^^^^^
╰-----------------------------
```

## not

```jspl
not(Bool) -> Bool
not(Int) -> Int
```

Returns the NOT of `Bool` or `Int`.

```jspl
not(true) => false
```

## and

```jspl
and(Bool, Bool...) -> Bool
and(Int, Int...) -> Int
```

Returns the AND of `Bool` or `Int`.

```jspl
true and false => false
```

## or

```jspl
or(Bool, Bool...) -> Bool
or(Int, Int...) -> Int
```

Returns the OR of `Bool` or `Int`.

```jspl
true or false => true
```

## xor

```jspl
xor(Bool, Bool...) -> Bool
xor(Int, Int...) -> Int
```

Returns the XOR of `Bool` or `Int`.

```jspl
true xor false => true
```
