# Jsompiler - JSON Syntax Programming Language

Jsompiler is a compiler for the JSON syntax programming language.

This program converts a program written in JSON into GNU Assembly, compiles it, and executes it.

🚨 This program runs only on Windows! 🚨

Prerequisites
Make sure the following tools are installed and included in your PATH environment variable:

---

- gcc

---

## Installation & Usage

```bash
git clone https://github.com/HAL-G1THuB/jsompiler
cd jsompiler
cargo run <input_json_file>
```

## Command Syntax

```bash
jsompiler <input_json_file>
```

📌 Replace <input_json_file> with your actual JSON file.

## example

```json
["begin", ["=", "a", "title"], ["message", ["$", "a"], "345"]]
```

Execution order:

The expressions are evaluated sequentially by "begin".

The variable "a" is assigned the string "title" using "=".

A message box appears with the title (variable "a") and the body ("345") due to "message".
