# Functions

## Notation

- `-> T` — Returns a value of type `T`  
- `=> V` — Evaluates to the value `V`  
- `L...` — A literal of type `...`  
- `"..."` — Zero or more arguments matching the previous pattern  

---

## begin

```json
["begin", {"expr": "Any"}, "...", {"return_value": "Any"}] -> {"return_value": "Any"}
```

Evaluates each expression in sequence and returns the value of the last one.

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

Subtracts all subsequent operands from the first operand and returns the result.

```json
["-", 30, 5, ["+", 4, 6]] => 15
```

---

## lambda

```json
["lambda", {"params": "empty [] (todo)"}, {"body": "Any"}, "..."] -> "Function"
```

Creates a function.  
The first argument is the parameter list, and the remaining arguments are treated as expressions in the function body.  
Returns the created function object.

```json
["lambda", [], ["+", 4, 6], "this function returns a string"]
```

---

## message

```json
["message", {"title": "String"}, {"text": "String"}] => 1
```

Displays a message box.  
The first argument specifies the title, and the second specifies the message body.  
Returns the ID of the button pressed (currently always `1`, corresponding to `IDOK` in C/C++).

---

## =

```json
["=", {"variable": "LString"}, {"value": "Any"}] -> {"value": "Any (non-Literal)"}
```

Assigns the value of the second argument to the variable named by the first argument.  
Returns the assigned value.

---

## $

```json
["$", {"variable": "LString"}] -> {"value": "Any (non-Literal)"}
```

Retrieves and returns the value associated with the specified variable.
