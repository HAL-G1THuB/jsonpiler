# Jsonpiler - JSON Syntax Programming Language

**Jsonpiler** is a compiler for the JSON syntax programming language and its compiler.

This program converts a JSON-based program to GNU assembly, compiles it, and executes the result.  

[Japanese(日本語)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/README-ja.md)

- [GitHub repository](https://github.com/HAL-G1THuB/jsonpiler)  
- [Crates.io](https://crates.io/crates/jsonpiler)  
- [AI-generated Docs ![badge](https://deepwiki.com/badge.svg)](https://deepwiki.com/HAL-G1THuB/jsonpiler)  
🚨 **This program only runs on Windows (x64)!** 🚨

## What's New

### 0.4.2

- Added a new function `concat` to concatenate string literals while preserving their literal nature.
- Split `Object` into three variants:
  - **HashMap**: a collection of key/value pairs.
  - **Sequence**: an ordered sequence of instructions.
  - **TypeAnnotations**: type annotations for variables or functions.
- Now generates an error if a quadratic function is called without arguments.
- Implemented support for `lambda` arguments.
- Enhanced the types of `lambda` return values for richer type information.
- The minimum length of arguments for `+`, `/`, `*`, `or`, `and`, and `xor` is now 2.
- The return value of `message` is now `Null`.

[Project History and Plans](https://github.com/HAL-G1THuB/jsonpiler/blob/main/CHANGELOG.md)

## Prerequisites

**Make sure the following tools are installed and available in your PATH environment variable:**

- `ld` (from MinGW-w64)  
- `as` (from MinGW-w64)  

**The following DLLs must be present in `C:\Windows\System32\` for the program to work correctly:**

- `kernel32.dll`  
- `user32.dll`  

## Installation & Usage

```bash
cargo install jsonpiler
jsonpiler (input_json_file (UTF-8)) [arguments of .exe ...]
```

Replace `(input_json_file)` with the actual JSON file you want to compile.

## Function Documentation

[Function Reference (Markdown)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/functions.md)

## Language Documentation

[Language Reference (Markdown)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/specification.md)

## Example

[Examples](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples)

```json
{ "=": ["a", "title"], "message": [{"$": "a"}, "345"], "+": [1, 2, 3] }
```

**Execution order:**

The jsonpiler code consists of a single JSON object.

Expressions are evaluated sequentially.

The `"="` key assigns the string `"title"` to the variable `a`.  

The `"message"` key then uses the value of `a` followed by the string `"345"` to display a message.  

Finally, the `"+"` key calculates the sum of `1`, `2`, and `3`, producing the result `6`.

The program returns `6` as the final value of the `{}` block.

If this program were loaded into a jsonpiler running in cargo, it would display something like this (returning 6, as mentioned above).

```plaintext
error: process didn't exit successfully: `jsonpiler.exe test.json` (exit code: 6)
```

This is not an unexpected error; it is normal behavior.

## Error or warning message format

```json
{ "message": ["title", { "$": "doesn't_exist" }] }
```

```text
Compilation error: Undefined variables: `doesn't_exist`
Error occurred on line: 1
Error position:
{ "message": ["title", { "$": "doesn't_exist" }] }
                              ^^^^^^^^^^^^^^^
```

## Execution

```mermaid
graph TD
  A[file.json] --> B{Jsonpiler}
  B -->|Parse| C([AST])
  C -->|Compile| D[file.s]
  D --> |Assembling with GNU AS| E[file.obj]
  E --> |Linking with GNU LD| F[file.exe]
  S[C:\Windows\System32\] --> KERNEL32[kernel32.dll] --> F
  S --> USER32[user32.dll] --> F
  F --> Exec[(Execution)]
```
