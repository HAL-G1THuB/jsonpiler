# Jsompiler - JSON Syntax Programming Language

Jsompiler is a compiler for the JSON syntax programming language.

This program converts a program written in JSON into GNU Assembly, compiles it, and executes the result.  
[crates.io](https://crates.io/crates/jsompiler)  
[docs.rs](https://docs.rs/jsompiler/latest/jsompiler)  
🚨 **This program only runs on Windows(x64)!** 🚨

## Prerequisites

**Make sure the following tools are installed and included in your PATH environment variable:**

- ld (MinGW-w64)

- as (MinGW-w64)

**The following DLLs must be present in C:\System32 for this program to work properly.**

- kernel32.dll

- user32.dll

- ucrtbase.dll

## Installation & Usage

```bash
git clone https://github.com/HAL-G1THuB/jsompiler
cd jsompiler
cargo run <input_json_file>
```

## Command Syntax

```bash
jsompiler (input_json_file)
```

Replace (input_json_file) with the actual JSON file you want to compile.

## example

```json
["begin", ["=", "a", "title"], ["message", ["$", "a"], "345"]]
```

Execution order:

Expressions inside 'begin' are evaluated sequentially.

The variable "a" is assigned the string "title" using "=".

A message box appears with the title (variable "a") and the body ("345") due to "message".

## functions

### begin

```json
["begin", "expr: any", "..."]
```

Evaluate the expression sequentially and return the last value.

```json
["begin", ["+", 1, 3]]
```

### +

```json
["+", "operand: -> int", "..."]
```

Add the operands and return the result.

```json
["+", 1, 5, ["+", 4, 6]]
```

### -

```json
["-", "operand: -> int", "..."]
```

Subtract the subsequent operands from the first operand and return the result.

```json
["-", 30, 5, ["+", 4, 6]]
```

### lambda

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
