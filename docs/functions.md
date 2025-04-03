# Functions

## begin

```json
["begin", "expr: any", "..."]
```

Evaluate the expression sequentially and return the last value.

```json
["begin", ["+", 1, 3]]
```

## +

```json
["+", "operand: -> int", "..."]
```

Add the operands and return the result.

```json
["+", 1, 5, ["+", 4, 6]]
```

## -

```json
["-", "operand: -> int", "..."]
```

Subtract the subsequent operands from the first operand and return the result.

```json
["-", 30, 5, ["+", 4, 6]]
```

## lambda

```json
["lambda", "params: empty [] (todo)", "expr: any", "..."]
```

Create a function where the first argument is the argument list,
and the remaining arguments are the content, then return the function.

```json
["lambda", [], ["+", 4, 6], "this function return string"]
```

## message

```json
["message", "title: string", "text: string"]
```

Create a message box where the first argument specifies the title and the second argument specifies the message body.
The function returns the ID of the pressed button.

## =

```json
["=", "variable: string", "value: any"]
```

Assign the second argument's value to the variable named in the first argument, then return the assigned value.

## $

```json
["$", "variable: string"]
```

Retrieve and return the value of the specified variable.
