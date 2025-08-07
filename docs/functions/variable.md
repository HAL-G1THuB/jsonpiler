# Variables and scope

## scope

```json
{"scope": {"expression": "Object"}} -> "Any"
```

It introduces a new scope and evaluates the expression in order and returns the result.

```json
{ "scope": {"+": [1, 3], "value": 1} } => 0
```

## = / global

```json
{"=": [{"variable": "LString"}, {"value": "Any"}]} -> "Null"
```

```json
{"global": [{"variable": "LString"}, {"value": "Any"}]} -> "Null"
```

Assigns the given value to the specified variable name.  
Returns `null` after assignment.
Assigned to local scope for `=` and to global scope for `global`.
Currently, the following types are **not assignable**:

- Array  
- Object  

**Reassignment is not yet implemented.**

## $

```json
{"$": {"variable": "LString"}} -> "VAny"
```

Returns the value bound to the given variable name.
