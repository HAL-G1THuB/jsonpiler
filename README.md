# Jsonpiler — JSON Syntax Programming Language

**Jsonpiler** is a compiler and runtime for a programming language that uses **JSON** or **JSPL (Jsonpiler Structured Programming Language)** as its syntax.  It converts a JSON-based program into **x86\_64 Windows PE** machine code, links it, and executes the result.
Jsonpiler bundles an assembler and linker purpose-built for its IR and PE output on Windows.

[日本語 README](https://github.com/HAL-G1THuB/jsonpiler/blob/main/README-ja.md)

- [GitHub](https://github.com/HAL-G1THuB/jsonpiler)
- [Crates.io](https://crates.io/crates/jsonpiler)
- [AI-generated docs: ![badge](https://deepwiki.com/badge.svg)](https://deepwiki.com/HAL-G1THuB/jsonpiler)
- [VSCode Extensions](https://marketplace.visualstudio.com/items?itemName=H4LVS.jsplsyntax)

> 🚨 **Windows only (x64)** — Jsonpiler targets 64-bit Windows and produces native PE executables.

---

## What’s New

### 0.6.4

- Fixed a problem in which the fifth and later arguments of user-defined functions were not recognized correctly.
- Four arithmetic operations `+`, `-`, `*`, `/` now support Float type
- Added function to truncate Float to integer: `Int`.
- Added I/O function: `print`.
- Fixed an error in `global` function.

See **[CHANGELOG](https://github.com/HAL-G1THuB/jsonpiler/blob/main/CHANGELOG.md)** for full history and plans.

---

## Requirements

No external toolchains or libraries are required.

**The following system DLLs must be available in `C:\\Windows\\System32\\`:**

- `kernel32.dll`
- `user32.dll`

These are present on standard Windows installations.

---

## Install & Run

```bash
cargo install jsonpiler

# Compile and execute a JSON program
jsonpiler <input.json> [args passed to the produced .exe]
```

- `<input.json>` must be UTF-8 encoded.
- Any additional arguments are forwarded to the generated executable at runtime.

---

## Language & Function References

- **Language Spec (Markdown):** [https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/specification.md](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/specification.md)
- **Function Reference (Markdown):** [https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/functions.md](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/functions.md)

---

## JSPL

With the introduction of JSPL (Jsonpiler Structured Programming Language),
Jsonpiler significantly improves human readability and writability,
which were challenging under the strict JSON-based syntax.
JSPL is designed to express function definitions,conditionals,
function calls, and variable assignments in a natural and intuitive form.
All JSPL code is internally transformed into the same JSON-based intermediate representation (IR),
ensuring full compatibility with the existing Jsonpiler compilation infrastructure
while making programs easier to write and understand.
For more details, see the language specification above.

| Differences from JSON         | JSON                                  | JSPL                                          |
| ----------------------------- | ------------------------------------- | --------------------------------------------- |
| **Curly braces `{}`**         | Required                              | Optional for top-level blocks                 |
| **Function call syntax**      | Explicit form like `{"sum": [1,2,3]}` | Natural syntax like `sum(1, 2, 3)`            |
| **Identifier notation**       | All keys must be quoted `"string"`    | Unquoted identifiers are allowed              |
| **Ternary-style syntax**      | Not supported                         | `1 + 10` → expanded to `{ "+": [1, 10] }`     |
| **Variable reference syntax** | Explicit form like `{"$": "name"}`    | Can be written as `$name`                     |
| **Comments**                  | Not allowed (by spec)                 | Supported via `# comment`                     |
| **Control structures**        | Written as functions                  | Syntactic sugar like `if(...)`, `define(...)` |

---

## Examples

Browse ready-to-run samples in **`examples/`**:
[https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples)

Minimal example:

```json
{ "=": ["a", "title"], "message": [{"$": "a"}, "345"], "+": [1, 2, 3] }
```

### Execution order

- A Jsonpiler program is a single JSON object whose keys are evaluated **sequentially**.
- `"="` assigns the string `"title"` to the variable `a`.
- `"message"` prints the value of `a` followed by `"345"`.
- `"+"` computes the sum of `1`, `2`, and `3`, i.e., **6**.

The program’s **final expression value** becomes the process **exit code**. Running under `cargo run` may look like this (Windows reports process exit code 6):

```text
error: process didn't exit successfully: `jsonpiler.exe test.json` (exit code: 6)
```

This is expected behavior and not an error in Jsonpiler itself.

---

## Diagnostics (Errors & Warnings)

**Input:**

```json
{ "message": ["title", { "$": "doesn't_exist" }] }
```

**Output:**

```text
Compilation error: Undefined variables: `doesn't_exist`
Error occurred on line: 1
Error position:
{ "message": ["title", { "$": "doesn't_exist" }] }
                              ^^^^^^^^^^^^^^^
```

---

## Pipeline Overview

```mermaid
graph TD
  subgraph Read
    A["file.json
{ &quot;+&quot;: [1, 2] }"] --> B{Jsonpiler}
  end
  subgraph Parse
    B --o C["AST
Json::Object([
  Json::String(&quot;+&quot;),
  Json::Array([
    Json::Int(1),
    Json::Int(2)
])])"]
    B --x PError[[ParseError]]
  end
  subgraph Compile
    C --x CError[[CompileError]]
    C --o E["Assembler IR
[
  ...Inst::MovQQ(Rax, 1),
  Inst::MovQQ(Rcx, 2),
  Inst::AddRR(Rax, Rcx)...
]"]
  end
  subgraph Evaluate
    C --o D["Json::Int(Temporary Value)"]
    C --x EError[[TypeError or ArityError]]
    D -->|Detect Exit Code| E
  end
  subgraph Assemble
    E --o G["Binary machine code
  [...0x48, 0x89, 0o201]..."]
    E --x AError[[InternalError]]
  end
  subgraph Link
    G --o F["Portable Executable (PE)"]
    G --x LError[[InternalError]]
  end
  subgraph Write
    F --> H[file.exe]
  end
  subgraph Execution
    H --> Exec[(Execute)]
  end
  subgraph DLL
    S[C:\\Windows\\System32\\]
    KERNEL32[kernel32.dll]
    USER32[user32.dll]
    S --> KERNEL32 --> F
    S --> USER32 --> F
  end
```

---

## Notes

- Output is a native **PE executable** for Windows x64.
- SEH is currently disabled and may be re-enabled in a future release.
- If you see a non-zero exit code under Cargo, it likely reflects your program’s final value.

---

## License

This project’s license is specified in the repository.

---

## Contributing

Issues and PRs are welcome! If you find a bug, please include the following information:

> 🚨 Please make sure you are running on Windows x64.

- The JSON program (minimal reproduction if possible)
- Jsonpiler version

---
