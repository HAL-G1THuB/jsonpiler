# Jsonpiler - JSON Syntax Programming Language

**Jsonpiler** is a compiler for the JSON Syntax Programming Language.

This program converts a JSON-based program into GNU Assembly, compiles it, and executes the result.  

- [GitHub Repository](https://github.com/HAL-G1THuB/jsonpiler.git)  
- [Crates.io](https://crates.io/crates/jsonpiler)  
- [Docs.rs](https://docs.rs/jsonpiler/latest/jsonpiler)  
- [Fallback documentation (if docs.rs fails)](https://hal-g1thub.github.io/jsonpiler-doc/jsonpiler/index.html)  
🚨 **This program only runs on Windows (x64)!** 🚨

## What's New

- **Objects now preserve insertion order.**
- **Object values are now evaluated in insertion order.**
- Add MerMaid to README.md
- Optimize message box functions.
- Fixed a bug in the evaluation order.

## Prerequisites

**Make sure the following tools are installed and available in your PATH environment variable:**

- `ld` (from MinGW-w64)  
- `as` (from MinGW-w64)  

**The following DLLs must be present in `C:\Windows\System32` for the program to work correctly:**

- `kernel32.dll`  
- `user32.dll`  
- `ucrtbase.dll`  

## Installation & Usage

```bash
cargo install jsonpiler
jsonpiler (input_json_file in UTF-8)
```

Replace `(input_json_file)` with the actual JSON file you want to compile.

## Example

```json
["begin", ["=", "a", "title"], ["message", ["$", "a"], "345"]]
```

**Execution order:**

The jsonpiler code consists of a single JSON object.

Expressions inside `begin` are evaluated sequentially.

The variable `"a"` is assigned the string `"title"` using `"="`.

A message box appears with the title (from variable `"a"`) and the body `"345"`, as specified by `"message"`.

The program returns the integer ID of the pressed button in the message box  
(currently only `1` is supported, which corresponds to `IDOK` in C/C++),  
as the final value of the `begin` block.

## Function Documentation

[Function Reference (Markdown)](https://github.com/HAL-G1THuB/jsonpiler/tree/main/docs/functions.md)

## Language Documentation

[Language Reference (Markdown)](https://github.com/HAL-G1THuB/jsonpiler/tree/main/docs/specification.md)

## Execution

```mermaid
graph TD
  A[file.json] --> B{Jsonpiler}
  B -->|Parse| C([AST])
  C -->|Compile| D[file.s]
  D --> |Assembling with GNU AS| E[file.obj]
  E --> |Linking with GNU LD| F[file.exe]
  S[C:\Windows\System32\] --> KERNEL32[kernel32.dll] --> F[file.exe]
  S --> USER32[user32.dll] --> F[file.exe]
  S --> UCRTBASE[ucrtbase.dll] --> F[file.exe]
  F --> Execute[(Execute!)]
```
