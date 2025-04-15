# Functions

## Notation

- `-> T` — Returns a value of type `T`  
- `=> V` — Evaluates to the value `V`  
- `L...` — Literal(s) of type `...`  
- `"..."` — Zero or more arguments following the previous pattern  

---

## begin

```json
["begin", {"expr": "Any"}, "...", {"return_value": "Any"}] -> {"return_value": "Any"}
```

Evaluates each expression sequentially and returns the value of the last one.

```json
["begin", ["+", 1, 3], 0] => 0
```

---

## +

```json
["+", {"augend": "Int"}, {"addend": "Int"}, "..."] -> {"return_value": "Int"}
```

Adds all operands and returns the result.

```json
["+", 1, 5, ["+", 4, 6]] => 16
```

---

## -

```json
["-", {"minuend": "Int"}, {"subtrahend": "Int"}, "..."] -> {"return_value": "Int"}
```

Subtracts all following operands from the first operand and returns the result.

```json
["-", 30, 5, ["+", 4, 6]] => 15
```

---

## lambda

```json
["lambda", {"params": "empty [] (todo)"}, {"body": "Any"}, "..."] -> "Function"
```

Creates a function. The first argument is the parameter list, and the rest are the function body expressions. Returns the created function.

```json
["lambda", [], ["+", 4, 6], "this function returns a string"]
```

---

## message

```json
["message", {"title": "String"}, {"text": "String"}] => 1
```

Creates a message box. The first argument is the title, and the second is the body text.  
Returns the ID of the button pressed (currently only `1` is supported, corresponding to `IDOK` in C/C++).

---

## =

```json
["=", {"variable": "LString"}, {"value": "Any"}] -> {"value": "AnyVariable"}
```

Assigns the second argument’s value to the variable named by the first argument. Returns the assigned value.

---

## $

```json
["$", {"variable": "LString"}] -> {"value": "AnyVariable"}
```

Retrieves and returns the value of the specified variable.
