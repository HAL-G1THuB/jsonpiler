# Variables and scope

## scope

```jspl
scope(Block) -> Any
```

It introduces a new scope and evaluates the expression in order and returns the result.

```jspl
scope({
  x = 1
  $x
}) => 1
```

## = / global

```jspl
Str (Literal) = Any
  -> Null
```

```jspl
Str (Literal) global Any
  -> Null
```

Assigns the given value to the specified variable name.  
Returns `null` after assignment.
Assigned to local scope for `=` and to global scope for `global`.
`global` and `=` support reassignment.

Currently, the following types are **not assignable** and **not reassignable**:

- Array  
- Object  

## $

```jspl
$Str (Literal) -> Any (Local or Global Variable)
```

Returns the value bound to the given variable name.
