# Variables and scope

## scope

```json
{"scope": {"expression": "Sequence"}} -> "Any"
```

```jspl
scope({sequence})
```

It introduces a new scope and evaluates the expression in order and returns the result.

```json
{ "scope": {"+": [1, 3], "value": 1} } => 1
```

## = / global

```json
{"=": [{"variable": "String (Literal)"}, {"value": "Any"}]} -> "Null"
```

```json
{"global": [{"variable": "String (Literal)"}, {"value": "Any"}]} -> "Null"
```

```jspl
variable = value
global(variable, value)
```

Assigns the given value to the specified variable name.  
Returns `null` after assignment.
Assigned to local scope for `=` and to global scope for `global`.
`global` and `=` support reassignment.

Currently, the following types are **not assignable** and **not reassignable**:

- Array  
- Object  

## $

```json
{"$": {"variable": "String (Literal)"}} -> "VAny"
```

```jspl
$variable
```

Returns the value bound to the given variable name.
