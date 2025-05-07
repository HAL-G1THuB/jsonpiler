# Functions

## Notation

- `-> T` - Returns a value of type `T`  
- `=> V` - Evaluates to the value `V`  
- `L...` - A literal of type `...`  
- `V...` - A non-literal value of type `...`  
- `"..."` - Zero or more arguments matching the previous pattern  

## `scope`

```json
{"scope": [{"expr": "Any"}, "...", {"return_value": "Any"}]} -> "Any"
```

It evaluates each expression in order and returns the result of the last one.
`scope` introduces a new scope.

```json
{ "scope": [{"+": [1, 3]}, 0] } => 0
```

## `+`

```json
{"+": [{ "operand": "Int..." }, "..."]} -> "VInt"
```

Returns the sum of all operands.
If given zero arguments, it returns the identity element (0).

```json
{ "+": [ 1, 5, ["+", 4, 6]] } => 16
```

## `-`

```json
{ "-": [{ "operand": "Int..." }, "..."] } -> "VInt"
```

Subtracts all following operands from the first one and returns the result.
If given zero arguments, it returns the identity element (0).
If given one argument, invert the sign.

```json
{"-": [30, 5, {"+": [4, 6]}]} => 15
```

## `*`

```json
{"*": [{"operand": "Int..."}, "..."]} -> "VInt"
```

Returns the result of multiplying operands.
If given zero arguments, it returns the identity element (1).

```json
{"*": [30, 5, {"+":[4, 6]}]} => 1500
```

## `/`

```json
{"/": [{"operand": "Int"}, {"operand": "Int"}, "..."]} -> "VInt"
```

Returns the result of dividing the first operand by all following operands.
If less than 1 arguments are given, an error occurs.
If the number to divide is zero, an error is generated at runtime or compile time.

```json
{"/": [30, 5, 6]} => 1
```

## `%`

```json
{"%": [{"operand": "Int"}, {"operand": "Int"}]} -> "VInt"
```

Returns the result of the remainder operation.

```json
{"%": [30, 7]} => 2
```

## `lambda`

```json
{"lambda": [{"params": "empty [] (todo)"}, {"body": "Any"}, "..."]} -> "Function"
```

Creates a function.  
The first argument specifies the parameter list;  
the remaining arguments form the function body and are evaluated when the function is called.
`lambda` introduces a new scope.

```json
{"lambda": [[], {"+": [4, 6]}, 1]}
```

## `message`

```json
{"message": [{"title": "String"}, {"text": "String"}]} => 1
```

Displays a message box.  
The first argument is the title; the second is the body text.  
Returns the ID of the button pressed - currently always `1` (equivalent to `IDOK` in C/C++).

## `=`/`global`

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

- LArray  
- LBool  
- LObject  

**Reassignment is not yet implemented.**

## `$`

```json
{"$": {"variable": "LString"}} -> "VAny"
```

Returns the value bound to the given variable name.

## `'`

```json
{"'": {"expr": "Any"}} -> {"unevaluated_expr": "Any"}
```

Returns the expression without evaluating it.
Can also be used as a comment.

## `eval`

```json
{"eval": {"expr": "Any"}} -> {"evaluated_expr": "Any"}
```

Evaluates the given expression and returns the result.

## list

```json
{"list": [{"expr": "Any"}, "..."]} -> "LArray"
```

The list function returns its evaluated arguments as an LArray.

```json
{"list": {"+": [3, 5]}} => [8]
```

## abs

```json
{"abs": {"expr": "Int"}} -> "VInt"
```

Returns the absolute value of the given integer.

```json
{"abs": -5} => 5
```
