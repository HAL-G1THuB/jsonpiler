# Functions

## Notation

- `-> T` — Returns a value of type `T`  
- `=> V` — Evaluates to the value `V`  
- `L...` — A literal of type `...`  
- `V...` — A non-literal value of type `...`  
- `"..."` — Zero or more arguments matching the previous pattern  

---

## `begin`

```json
["begin", {"expr": "Any"}, "...", {"return_value": "Any"}] -> {"return_value": "Any"}
```

Evaluates each expression in order and returns the result of the last one.

```json
["begin", ["+", 1, 3], 0] => 0
```

---

## `+`

```json
["+", {"augend": "Int"}, {"addend": "Int"}, "..."] -> {"return_value": "VInt"}
```

Returns the sum of all operands.

```json
["+", 1, 5, ["+", 4, 6]] => 16
```

---

## `-`

```json
["-", {"minuend": "Int"}, {"subtrahend": "Int"}, "..."] -> {"return_value": "VInt"}
```

Subtracts all following operands from the first one and returns the result.

```json
["-", 30, 5, ["+", 4, 6]] => 15
```

---

## `lambda`

```json
["lambda", {"params": "empty [] (todo)"}, {"body": "Any"}, "..."] -> "Function"
```

Creates a function.  
The first argument specifies the parameter list; the remaining arguments are evaluated as expressions within the function body.  
Returns the resulting function object.
`lambda` has scope.

```json
["lambda", [], ["+", 4, 6], "this function returns a string"]
```

---

## `message`

```json
["message", {"title": "String"}, {"text": "String"}] => 1
```

Displays a message box.  
The first argument is the title; the second is the body text.  
Returns the ID of the button pressed — currently always `1` (equivalent to `IDOK` in C/C++).

---

## `=`

```json
["=", {"variable": "LString"}, {"value": "Any"}] -> "Null"
```

Assigns the given value to the specified variable name.  
Returns the assigned value.

Currently, the following types are **not assignable**:

- LArray  
- LBool  
- LFloat  
- LObject  

**Reassignment is not yet implemented.**

---

## `$`

```json
["$", {"variable": "LString"}] -> {"value": "VAny"}
```

Returns the value bound to the given variable name.

---
