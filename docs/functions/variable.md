# Variables and scope

## scope

```jspl
scope(Any) -> Any
```

It introduces a new scope and evaluates the expression in order and returns the result.

```jspl
scope(let(x = 1))
x

=>

╭- CompilationError ----------
| Undefined variable:
|   x
|-----------------------------
| input.jspl:2:1
|-----------------------------
| x
| ^
╰-----------------------------
```

## let

```jspl
let(Ident = Any) -> Null
```

Creates a **local variable** with the specified name and assigns the given value.
Returns `null` after creation.

Currently, the following types are **not assignable**:

- Array
- Object

## global

```jspl
global(Ident = Any) -> Null
```

Creates a **global variable** with the specified name and assigns the given value.
Returns `null` after creation.

Currently, the following types are **not assignable**:

- Array
- Object

## =

```jspl
Ident = Any -> Null
```

Reassigns a value to an **existing variable**.
Returns `null` after assignment.

`=` is used **only for reassignment** and does not create variables.

Currently, the following types are **not reassignable**:

- Array
- Object

## $

```jspl
Ident -> Any (Variable)
```

Returns the value bound to the given variable name.

```jspl
x
y

$(abc)

あいう
```
