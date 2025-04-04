# Jsompiler - JSON Syntax Programming Language

Jsompiler is a compiler for the JSON syntax programming language.

This program converts a program written in JSON into GNU Assembly, compiles it, and executes the result.  
[crates.io](https://crates.io/crates/jsompiler)  
[docs.rs](https://docs.rs/jsompiler/latest/jsompiler)  
[Documentation for when docs.rs builds fail.](https://hal-g1thub.github.io/jsompiler-doc/jsompiler/index.html)  
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
cargo install jsompiler
cd jsompiler
cargo run (input_json_file (utf-8))
```

## Command Syntax

```bash
jsompiler (input_json_file (utf-8))
```

Replace (input_json_file) with the actual JSON file you want to compile.

## Example

```json
["begin", ["=", "a", "title"], ["message", ["$", "a"], "345"]]
```

Execution order:

The jsompiler code consists of a single json object.

Expressions inside 'begin' are evaluated sequentially.

The variable "a" is assigned the string "title" using "=".

A message box appears with the title (variable "a") and the body ("345") due to "message".

## Document of functions

[Markdown](https://github.com/HAL-G1THuB/jsompiler/tree/main/docs/functions.md)
