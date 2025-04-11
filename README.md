# Jsompiler - JSON Syntax Programming Language

**Jsompiler** is a compiler for the JSON Syntax Programming Language.

This program converts a JSON-based program into GNU Assembly, compiles it, and executes the result.  

- [GitHub Repository](https://github.com/HAL-G1THuB/jsompiler.git)  
- [Crates.io](https://crates.io/crates/jsompiler)  
- [Docs.rs](https://docs.rs/jsompiler/latest/jsompiler)  
- [Fallback documentation (if docs.rs fails)](https://hal-g1thub.github.io/jsompiler-doc/jsompiler/index.html)  
🚨 **This program only runs on Windows (x64)!** 🚨

## What's New

- The program now sets the **exit code** to the return value when the entire program evaluates to an `int`.
- Lambda functions now return a value **only if** their return type is `int`.

## Prerequisites

**Make sure the following tools are installed and available in your PATH environment variable:**

- `ld` (from MinGW-w64)  
- `as` (from MinGW-w64)  

**The following DLLs must be present in `C:\System32` for the program to work correctly:**

- `kernel32.dll`  
- `user32.dll`  
- `ucrtbase.dll`  

## Installation & Usage

```bash
cargo install jsompiler
cd jsompiler
cargo run --release -- (input_json_file in UTF-8)
```

## Command Syntax

```bash
jsompiler (input_json_file in UTF-8)
```

Replace `(input_json_file)` with the actual JSON file you want to compile.

## Example

```json
["begin", ["=", "a", "title"], ["message", ["$", "a"], "345"]]
```

**Execution order:**

The jsompiler code consists of a single JSON object.

Expressions inside `begin` are evaluated sequentially.

The variable `"a"` is assigned the string `"title"` using `"="`.

A message box appears with the title (from variable `"a"`) and the body `"345"`, as specified by `"message"`.

The program returns the integer ID of the pressed button in the message box  
(currently only `1` is supported, which corresponds to `IDOK` in C/C++),  
as the final value of the `begin` block.

## Function Documentation

[Function Reference (Markdown)](https://github.com/HAL-G1THuB/jsompiler/tree/main/docs/functions.md)
